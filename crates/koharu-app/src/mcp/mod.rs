pub mod server;
pub mod tools;
pub mod types;

use anyhow::Result;
use tauri::{AppHandle, Cef};
use tokio::sync::RwLock;

pub use server::McpServer;

use crate::commands::preferences::McpConfig;

pub struct McpServerState {
    handle: AppHandle<Cef>,
    server: RwLock<Option<McpServer>>,
}

impl McpServerState {
    pub fn new(handle: AppHandle<Cef>) -> Self {
        Self {
            handle,
            server: RwLock::new(None),
        }
    }

    pub async fn update(&self, config: &McpConfig) -> Result<()> {
        let mut server_guard = self.server.write().await;
        if config.enabled {
            let need_restart = match server_guard.as_ref() {
                Some(current) => current.port() != config.port,
                None => true,
            };
            if need_restart {
                if let Some(current) = server_guard.take() {
                    current.stop();
                }
                let new_server = McpServer::start(self.handle.clone(), config.port).await?;
                *server_guard = Some(new_server);
            }
        } else if let Some(current) = server_guard.take() {
            current.stop();
        }
        Ok(())
    }

    pub async fn stop(&self) {
        let mut server_guard = self.server.write().await;
        if let Some(current) = server_guard.take() {
            current.stop();
        }
    }

    #[allow(dead_code)]
    pub async fn is_running(&self) -> bool {
        self.server.read().await.is_some()
    }
}
