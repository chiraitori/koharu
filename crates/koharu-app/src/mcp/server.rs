use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use serde_json::json;
use tauri::{AppHandle, Cef};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc, watch};

use super::tools::{execute_tool, list_tools};
use super::types::{JsonRpcRequest, JsonRpcResponse, McpToolResult};

type SseSessionMap = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

pub struct McpServer {
    port: u16,
    shutdown_tx: watch::Sender<bool>,
}

impl McpServer {
    pub async fn start(handle: AppHandle<Cef>, port: u16) -> Result<Self> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("failed to bind MCP server to 127.0.0.1:{port}"))?;

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let sessions: SseSessionMap = Arc::new(Mutex::new(HashMap::new()));

        tracing::info!(port, "Koharu MCP server started on 127.0.0.1:{port}");

        let server_sessions = sessions.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            tracing::info!("Koharu MCP server stopping");
                            break;
                        }
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, client_addr)) => {
                                let app = handle.clone();
                                let sessions = server_sessions.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, app, sessions).await {
                                        tracing::debug!(?client_addr, error = %e, "MCP client connection closed with error");
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "MCP listener accept failed");
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self { port, shutdown_tx })
    }

    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    handle: AppHandle<Cef>,
    sessions: SseSessionMap,
) -> Result<()> {
    let mut buffer = Vec::with_capacity(4096);
    let mut temp = [0u8; 4096];

    let header_end;
    loop {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&temp[..n]);

        if let Some(pos) = find_subsequence(&buffer, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if let Some(pos) = find_subsequence(&buffer, b"\n\n") {
            header_end = pos + 2;
            break;
        }
        if buffer.len() > 65536 {
            send_http_error(&mut stream, 431, "Request Header Fields Too Large").await?;
            return Ok(());
        }
    }

    let header_str = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_str.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let uri = parts.next().unwrap_or_default();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    // CORS preflight
    if method.eq_ignore_ascii_case("OPTIONS") {
        let response = "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: Content-Type, Authorization, x-session-id\r\n\
Access-Control-Max-Age: 86400\r\n\
Connection: close\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    let (path, query) = match uri.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (uri, None),
    };

    let session_id_from_query = query.and_then(|q| {
        q.split('&').find_map(|pair| {
            pair.split_once('=').and_then(|(k, v)| {
                if k == "sessionId" {
                    Some(v.to_string())
                } else {
                    None
                }
            })
        })
    });

    // Health check
    if (path == "/" || path == "/health") && method.eq_ignore_ascii_case("GET") {
        let body = json!({
            "status": "ok",
            "server": "koharu-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }).to_string();
        send_json_response(&mut stream, 200, &body).await?;
        return Ok(());
    }

    // SSE Transport: GET /mcp
    if path == "/mcp" && method.eq_ignore_ascii_case("GET") {
        let session_id = uuid::Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        {
            let mut s = sessions.lock().await;
            s.insert(session_id.clone(), tx);
        }

        let init_response = format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: text/event-stream\r\n\
Cache-Control: no-cache\r\n\
Connection: keep-alive\r\n\
Access-Control-Allow-Origin: *\r\n\r\n\
event: endpoint\r\ndata: /mcp/messages?sessionId={session_id}\r\n\r\n"
        );
        stream.write_all(init_response.as_bytes()).await?;

        let keepalive_interval = Duration::from_secs(15);
        let mut interval = tokio::time::interval(keepalive_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if stream.write_all(b": ping\n\n").await.is_err() {
                        break;
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(data) => {
                            let sse_msg = format!("event: message\r\ndata: {data}\r\n\r\n");
                            if stream.write_all(sse_msg.as_bytes()).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        let mut s = sessions.lock().await;
        s.remove(&session_id);
        return Ok(());
    }

    // JSON-RPC Transport: POST /mcp or POST /mcp/messages
    if (path == "/mcp" || path == "/mcp/messages") && method.eq_ignore_ascii_case("POST") {
        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let mut body_bytes = buffer[header_end..].to_vec();
        while body_bytes.len() < content_length {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            body_bytes.extend_from_slice(&temp[..n]);
        }

        let body_str = String::from_utf8_lossy(&body_bytes);
        let request: JsonRpcRequest = match serde_json::from_str(&body_str) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {e}"));
                send_json_response(&mut stream, 200, &serde_json::to_string(&err_resp)?).await?;
                return Ok(());
            }
        };

        let response = handle_jsonrpc(&handle, request).await;
        if let Some(resp) = response {
            let resp_str = serde_json::to_string(&resp)?;

            // Forward to active SSE stream if connected via session ID
            if let Some(sid) = session_id_from_query {
                let s = sessions.lock().await;
                if let Some(tx) = s.get(&sid) {
                    let _ = tx.send(resp_str.clone());
                }
            }

            send_json_response(&mut stream, 200, &resp_str).await?;
        } else {
            // Notification handled, empty 202 Accepted
            let empty_resp = "HTTP/1.1 202 Accepted\r\n\
Access-Control-Allow-Origin: *\r\n\
Content-Length: 0\r\n\
Connection: close\r\n\r\n";
            stream.write_all(empty_resp.as_bytes()).await?;
        }

        return Ok(());
    }

    send_http_error(&mut stream, 404, "Not Found").await?;
    Ok(())
}

async fn handle_jsonrpc(
    handle: &AppHandle<Cef>,
    request: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let id = request.id;
    match request.method.as_str() {
        "initialize" => {
            Some(JsonRpcResponse::success(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "koharu",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            ))
        }
        "notifications/initialized" => None,
        "ping" => Some(JsonRpcResponse::success(id, json!({}))),
        "tools/list" => {
            let tools = list_tools();
            Some(JsonRpcResponse::success(id, json!({ "tools": tools })))
        }
        "tools/call" => {
            let name = request.params.get("name").and_then(|v| v.as_str());
            let arguments = request.params.get("arguments").cloned().unwrap_or(json!({}));

            let Some(name) = name else {
                return Some(JsonRpcResponse::error(id, -32602, "Missing 'name' in tool call"));
            };

            match execute_tool(handle, name, arguments).await {
                Ok(result) => Some(JsonRpcResponse::success(id, serde_json::to_value(result).unwrap_or(json!({})))),
                Err(err) => {
                    let error_result = McpToolResult::error(format!("{:#}", err));
                    Some(JsonRpcResponse::success(id, serde_json::to_value(error_result).unwrap_or(json!({}))))
                }
            }
        }
        unknown => {
            Some(JsonRpcResponse::error(
                id,
                -32601,
                format!("Method '{unknown}' not found"),
            ))
        }
    }
}

async fn send_json_response(stream: &mut TcpStream, status: u16, body: &str) -> Result<()> {
    let status_text = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
Content-Type: application/json\r\n\
Access-Control-Allow-Origin: *\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\r\n\
{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn send_http_error(stream: &mut TcpStream, status: u16, message: &str) -> Result<()> {
    let body = format!(r#"{{"error":"{message}"}}"#);
    send_json_response(stream, status, &body).await
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}
