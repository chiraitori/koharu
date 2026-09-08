use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context as _, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use koharu_agent::{Control, Host, ToolCall};
use koharu_desktop::{Desktop, Frame};
use koharu_scene::{AssetInput, AssetMetadata, AssetRole, At, EntityId, PageDraft};
use serde_json::{Value, json};
use tauri::{AppHandle, Cef, Manager as _};
use walkdir::WalkDir;

use super::types::{McpContent, McpToolDefinition, McpToolResult};
use crate::commands::{
    ChannelExt as _,
    agent::host::KoharuHost,
    canvas::CanvasChannel,
    import::{self, Format},
    lifecycle::{self, AgentActivityGuard, ProjectChannel},
    preferences::Preferences,
    processing::{JobChannel, Processing},
    project::{CurrentProject, Project, ProjectLibrary, RasterStrokeMode},
};

pub fn list_tools() -> &'static [McpToolDefinition] {
    static TOOLS: OnceLock<Vec<McpToolDefinition>> = OnceLock::new();
    TOOLS.get_or_init(|| {
        vec![
            McpToolDefinition {
                name: "inspect_project".to_string(),
                description: "Read the latest semantic state of the currently open project. Includes project info, page list, dimensions, layers (detected text bubbles, translations, coordinates), available fonts, and pipeline settings.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "view_page".to_string(),
                description: "Render and view a specific page as an image. Returns visual WebP image data for visual inspection, bubble localization, and verification.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page": {
                            "type": "string",
                            "description": "The page entity ID to render and view"
                        }
                    },
                    "required": ["page"]
                }),
            },
            McpToolDefinition {
                name: "import_images".to_string(),
                description: "Import image files or folders into the active project. Supports PNG, JPG, WebP, ZIP, CBZ, RAR, and PDF.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "List of absolute file or directory paths to import"
                        }
                    },
                    "required": ["paths"]
                }),
            },
            McpToolDefinition {
                name: "run_pipeline".to_string(),
                description: "Run Koharu's automated processing pipeline: text detection, OCR, translation, or inpainting for the project, selected pages, or selected text elements.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "enum": ["full", "detection", "ocr", "translation", "inpainting"],
                            "description": "Pipeline stage to run: full, detection, ocr, translation, or inpainting"
                        },
                        "pages": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional list of page entity IDs to process (default: whole project)"
                        },
                        "elements": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional list of element entity IDs to process"
                        }
                    },
                    "required": ["operation"]
                }),
            },
            McpToolDefinition {
                name: "set_translation".to_string(),
                description: "Set or update the translated text for a text bubble element, or pass null to clear it.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "element": {
                            "type": "string",
                            "description": "The element ID"
                        },
                        "text": {
                            "type": ["string", "null"],
                            "description": "The translated text, or null to clear"
                        }
                    },
                    "required": ["element"]
                }),
            },
            McpToolDefinition {
                name: "set_source_text".to_string(),
                description: "Set or replace an element's source (OCR) text.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "element": {
                            "type": "string",
                            "description": "The element ID"
                        },
                        "text": {
                            "type": "string",
                            "description": "The updated source text"
                        }
                    },
                    "required": ["element", "text"]
                }),
            },
            McpToolDefinition {
                name: "add_text_box".to_string(),
                description: "Add a new text box or speech bubble to a page at the specified coordinates.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page": {
                            "type": "string",
                            "description": "The page entity ID"
                        },
                        "x": {
                            "type": "number",
                            "description": "X coordinate in page space"
                        },
                        "y": {
                            "type": "number",
                            "description": "Y coordinate in page space"
                        },
                        "width": {
                            "type": "number",
                            "description": "Width in page space"
                        },
                        "height": {
                            "type": "number",
                            "description": "Height in page space"
                        },
                        "angle_degrees": {
                            "type": "number",
                            "description": "Optional rotation angle in degrees (default 0)"
                        }
                    },
                    "required": ["page", "x", "y", "width", "height"]
                }),
            },
            McpToolDefinition {
                name: "set_typography".to_string(),
                description: "Update typography settings for an element (font, size, weight, style, color, alignment, writing mode).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "element": { "type": "string" },
                        "preferred_font": { "type": "string", "description": "Font family name from available_fonts" },
                        "font_weight": { "type": "integer", "description": "Font weight (e.g. 400, 700)" },
                        "font_style": { "type": "string", "enum": ["normal", "italic", "oblique"] },
                        "size": { "type": "number", "description": "Font size" },
                        "auto_fit": { "type": "boolean", "description": "Whether to auto-scale font size to fit" },
                        "color": { "type": "array", "items": { "type": "integer" }, "description": "[R, G, B, A] 0-255" },
                        "stroke_color": { "type": "array", "items": { "type": "integer" }, "description": "[R, G, B, A] 0-255" },
                        "stroke_width": { "type": "number" },
                        "alignment": { "type": "string", "enum": ["start", "center", "end", "justify"] },
                        "writing_mode": { "type": "string", "enum": ["horizontal", "vertical"] }
                    },
                    "required": ["element"]
                }),
            },
            McpToolDefinition {
                name: "set_geometry".to_string(),
                description: "Replace an element's page-space polygon geometry points.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "element": { "type": "string" },
                        "points": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "x": { "type": "number" },
                                    "y": { "type": "number" }
                                },
                                "required": ["x", "y"]
                            }
                        }
                    },
                    "required": ["element", "points"]
                }),
            },
            McpToolDefinition {
                name: "set_visibility".to_string(),
                description: "Change visibility and/or opacity for one or more elements.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "elements": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "visible": { "type": "boolean" },
                        "opacity": { "type": "number" }
                    },
                    "required": ["elements"]
                }),
            },
            McpToolDefinition {
                name: "delete_elements".to_string(),
                description: "Delete elements and their descendants from the scene.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "elements": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["elements"]
                }),
            },
            McpToolDefinition {
                name: "rename_page".to_string(),
                description: "Rename a project page.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page": { "type": "string" },
                        "label": { "type": "string" }
                    },
                    "required": ["page", "label"]
                }),
            },
            McpToolDefinition {
                name: "move_page".to_string(),
                description: "Move a page to a new zero-based index in the project page list.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page": { "type": "string" },
                        "index": { "type": "integer" }
                    },
                    "required": ["page", "index"]
                }),
            },
            McpToolDefinition {
                name: "delete_pages".to_string(),
                description: "Delete pages and their contents from the project.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pages": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["pages"]
                }),
            },
            McpToolDefinition {
                name: "open_project".to_string(),
                description: "Open an existing project from Koharu's library by name.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "The name of the project to open"
                        }
                    },
                    "required": ["name"]
                }),
            },
            McpToolDefinition {
                name: "create_project".to_string(),
                description: "Create and open a new empty project in Koharu's library.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "The name for the new project"
                        }
                    },
                    "required": ["name"]
                }),
            },
            McpToolDefinition {
                name: "list_projects".to_string(),
                description: "List all projects in Koharu's library.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "move_element".to_string(),
                description: "Move an element under a page or element at a zero-based index.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "element": { "type": "string" },
                        "parent": { "type": "string" },
                        "index": { "type": "integer" }
                    },
                    "required": ["element", "parent", "index"]
                }),
            },
            McpToolDefinition {
                name: "undo".to_string(),
                description: "Undo the last action in the active project.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "redo".to_string(),
                description: "Redo the previously undone action in the active project.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "select_page".to_string(),
                description: "Select the active page by entity ID.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page": { "type": "string" }
                    },
                    "required": ["page"]
                }),
            },
            McpToolDefinition {
                name: "close_project".to_string(),
                description: "Close the currently open project.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "delete_project".to_string(),
                description: "Delete a project from the library by name. Closes it first if it is open.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"]
                }),
            },
            McpToolDefinition {
                name: "export_pages".to_string(),
                description: "Export pages to PNG or PSD files in an existing directory. Defaults to all pages.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "directory": { "type": "string", "description": "Absolute path to an existing directory" },
                        "format": { "type": "string", "enum": ["png", "psd"] },
                        "pages": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["directory", "format"]
                }),
            },
            McpToolDefinition {
                name: "add_point_text".to_string(),
                description: "Add point text on the active page at x/y in page space. Returns the new element ID.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "x": { "type": "number" },
                        "y": { "type": "number" }
                    },
                    "required": ["x", "y"]
                }),
            },
            McpToolDefinition {
                name: "commit_paint".to_string(),
                description: "Paint a raster stroke on the active page. Creates a paint layer when layer is omitted.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "layer": { "type": ["string", "null"] },
                        "points": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": { "x": { "type": "number" }, "y": { "type": "number" } },
                                "required": ["x", "y"]
                            }
                        },
                        "diameter": { "type": "number" },
                        "color": { "type": "array", "items": { "type": "integer" }, "description": "[R,G,B,A] 0-255" }
                    },
                    "required": ["points", "diameter", "color"]
                }),
            },
            McpToolDefinition {
                name: "commit_erase".to_string(),
                description: "Erase part of a raster paint layer on the active page.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "layer": { "type": "string" },
                        "points": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": { "x": { "type": "number" }, "y": { "type": "number" } },
                                "required": ["x", "y"]
                            }
                        },
                        "diameter": { "type": "number" }
                    },
                    "required": ["layer", "points", "diameter"]
                }),
            },
            McpToolDefinition {
                name: "commit_transform".to_string(),
                description: "Move/resize/rotate text elements by giving new frames in page space.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "elements": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "element": { "type": "string" },
                                    "x": { "type": "number" },
                                    "y": { "type": "number" },
                                    "width": { "type": "number" },
                                    "height": { "type": "number" },
                                    "angle_degrees": { "type": "number" }
                                },
                                "required": ["element", "x", "y", "width", "height"]
                            }
                        }
                    },
                    "required": ["elements"]
                }),
            },
            McpToolDefinition {
                name: "request_inpaint".to_string(),
                description: "Run inpainting for a brushed region on a page. Returns the pipeline job ID.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page": { "type": "string" },
                        "points": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": { "x": { "type": "number" }, "y": { "type": "number" } },
                                "required": ["x", "y"]
                            }
                        },
                        "diameter": { "type": "number" }
                    },
                    "required": ["page", "points", "diameter"]
                }),
            },
            McpToolDefinition {
                name: "stop_processing".to_string(),
                description: "Stop all running pipeline jobs.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "get_fonts".to_string(),
                description: "List available fonts for typography.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "get_translation_models".to_string(),
                description: "List available translation models.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "get_preferences".to_string(),
                description: "Read pipeline, provider, typesetting, MCP, and language preferences.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpToolDefinition {
                name: "get_page_thumbnail".to_string(),
                description: "Get a small WebP thumbnail of a page source image as base64 image data.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page": { "type": "string" }
                    },
                    "required": ["page"]
                }),
            },
        ]
    })
}

pub async fn execute_tool(
    handle: &AppHandle<Cef>,
    name: &str,
    arguments: Value,
) -> Result<McpToolResult> {
    match name {
        "import_images" => {
            let _guard = AgentActivityGuard::new(handle.clone(), None, name);
            import_images(handle, arguments).await
        }
        "open_project" => {
            let _guard = AgentActivityGuard::new(handle.clone(), None, name);
            open_project(handle, arguments).await
        }
        "create_project" => {
            let _guard = AgentActivityGuard::new(handle.clone(), None, name);
            create_project(handle, arguments).await
        }
        "list_projects" => list_projects(handle).await,
        "select_page" => {
            let page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), page, name);
            select_page(handle, arguments).await
        }
        "close_project" => {
            let _guard = AgentActivityGuard::new(handle.clone(), None, name);
            close_project(handle).await
        }
        "delete_project" => {
            let _guard = AgentActivityGuard::new(handle.clone(), None, name);
            delete_project(handle, arguments).await
        }
        "export_pages" => {
            let _guard = AgentActivityGuard::new(handle.clone(), None, name);
            export_pages(handle, arguments).await
        }
        "add_point_text" => {
            let page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), page, name);
            add_point_text(handle, arguments).await
        }
        "commit_paint" => {
            let page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), page, name);
            commit_paint(handle, arguments).await
        }
        "commit_erase" => {
            let page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), page, name);
            commit_erase(handle, arguments).await
        }
        "commit_transform" => {
            let page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), page, name);
            commit_transform(handle, arguments).await
        }
        "request_inpaint" => {
            let page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), page, name);
            request_inpaint(handle, arguments).await
        }
        "stop_processing" => stop_processing(handle).await,
        "get_fonts" => get_fonts(handle).await,
        "get_translation_models" => get_translation_models().await,
        "get_preferences" => get_preferences().await,
        "get_page_thumbnail" => get_page_thumbnail(handle, arguments).await,
        "undo" => {
            let active_page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), active_page, name);
            undo(handle).await
        }
        "redo" => {
            let active_page = handle
                .state::<CurrentProject>()
                .project
                .lock()
                .await
                .as_ref()
                .and_then(|p| p.active_page());
            let _guard = AgentActivityGuard::new(handle.clone(), active_page, name);
            redo(handle).await
        }
        _ => {
            let host = KoharuHost::new(handle.clone());
            let control = Control::default();
            let call = ToolCall {
                call_id: "mcp-call".to_string(),
                name: name.to_string(),
                arguments: serde_json::to_string(&arguments)?,
            };

            let invocation = host.invoke(call, &control).await?;
            let mut contents = Vec::new();

            for image in invocation.images {
                let (mime, data) = if let Some(stripped) = image.data_url.strip_prefix("data:image/webp;base64,") {
                    ("image/webp", stripped.to_string())
                } else if let Some(stripped) = image.data_url.strip_prefix("data:image/png;base64,") {
                    ("image/png", stripped.to_string())
                } else if let Some(stripped) = image.data_url.strip_prefix("data:image/jpeg;base64,") {
                    ("image/jpeg", stripped.to_string())
                } else {
                    ("image/webp", image.data_url)
                };

                contents.push(McpContent::Image {
                    mime_type: mime.to_string(),
                    data,
                });
            }

            let text = serde_json::to_string_pretty(&invocation.value)
                .unwrap_or_else(|_| invocation.value.to_string());
            contents.push(McpContent::Text { text });

            Ok(McpToolResult {
                content: contents,
                is_error: false,
            })
        }
    }
}

async fn import_images(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let paths: Vec<String> = serde_json::from_value(
        arguments
            .get("paths")
            .cloned()
            .context("missing 'paths' argument")?,
    )?;

    if paths.is_empty() {
        bail!("paths list cannot be empty");
    }

    let mut files = Vec::new();
    for path_str in paths {
        let p = PathBuf::from(&path_str);
        if !p.exists() {
            bail!("path does not exist: {}", p.display());
        }
        if p.is_dir() {
            for entry in WalkDir::new(&p).follow_links(false).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension().and_then(|ext| ext.to_str()) {
                        if ext.parse::<Format>().is_ok() {
                            files.push(entry.into_path());
                        }
                    }
                }
            }
        } else if p.is_file() {
            files.push(p);
        }
    }

    if files.is_empty() {
        bail!("no supported image, archive, or PDF files found in the provided paths");
    }

    let pages = tokio::task::spawn_blocking(move || import::import(files))
        .await
        .context("page import worker stopped unexpectedly")??;
    let page_count = pages.len();
    let page_names: Vec<String> = pages.iter().map(|p| p.name.clone()).collect();

    let current = handle.state::<CurrentProject>();
    let (commit, page, info) = {
        let mut project = current.project.lock().await;
        let project = project.as_mut().context("no project is open")?;
        let source = AssetRole::new("source")?;
        let patch = project.snapshot().patch(|edit| {
            for imported in pages {
                let page = edit.add_page(
                    PageDraft::new(
                        imported.name,
                        f64::from(imported.width),
                        f64::from(imported.height),
                    ),
                    At::End,
                )?;
                edit.set_asset(
                    page,
                    &source,
                    AssetInput::new(
                        imported.bytes,
                        imported.format.to_mime_type(),
                        AssetMetadata {
                            width: Some(imported.width),
                            height: Some(imported.height),
                            attributes: Default::default(),
                        },
                    ),
                )?;
            }
            Ok(())
        })?;
        let commit = project.session.commit(patch).await?;
        project.record(vec![commit.revision]);
        project.reconcile_page();
        let page = project.active_page();
        let info = project.info();
        (commit, page, info)
    };

    let desktop = handle.state::<Desktop>();
    desktop.synchronize(&commit.snapshot, page, &commit).await?;
    let canvas = desktop.canvas_state();
    handle.state::<CanvasChannel>().channel.publish(canvas);
    handle.state::<ProjectChannel>().channel.publish(Some(info));

    Ok(McpToolResult::json(&json!({
        "success": true,
        "imported_pages_count": page_count,
        "pages": page_names
    })))
}

async fn open_project(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let name: String = serde_json::from_value(
        arguments
            .get("name")
            .cloned()
            .context("missing 'name' argument")?,
    )?;

    lifecycle::open_project(name.clone(), handle.clone())
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    Ok(McpToolResult::json(&json!({
        "success": true,
        "opened_project": name
    })))
}

async fn create_project(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let name: String = serde_json::from_value(
        arguments
            .get("name")
            .cloned()
            .context("missing 'name' argument")?,
    )?;

    lifecycle::create_project(name.clone(), handle.clone())
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    Ok(McpToolResult::json(&json!({
        "success": true,
        "created_project": name
    })))
}

async fn list_projects(handle: &AppHandle<Cef>) -> Result<McpToolResult> {
    let library = handle.state::<ProjectLibrary>().inner().clone();
    let projects = library.list()?;
    Ok(McpToolResult::json(&projects))
}

async fn undo(handle: &AppHandle<Cef>) -> Result<McpToolResult> {
    let desktop = handle.state::<Desktop>();
    let project = handle.state::<CurrentProject>();
    let canvas_channel = handle.state::<CanvasChannel>();

    crate::commands::editing::undo(desktop, project, canvas_channel)
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    if let Some(info) = handle
        .state::<CurrentProject>()
        .project
        .lock()
        .await
        .as_ref()
        .map(Project::info)
    {
        handle.state::<ProjectChannel>().channel.publish(Some(info));
    }

    Ok(McpToolResult::json(&json!({
        "success": true,
        "action": "undo"
    })))
}

async fn redo(handle: &AppHandle<Cef>) -> Result<McpToolResult> {
    let desktop = handle.state::<Desktop>();
    let project = handle.state::<CurrentProject>();
    let canvas_channel = handle.state::<CanvasChannel>();

    crate::commands::editing::redo(desktop, project, canvas_channel)
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    if let Some(info) = handle
        .state::<CurrentProject>()
        .project
        .lock()
        .await
        .as_ref()
        .map(Project::info)
    {
        handle.state::<ProjectChannel>().channel.publish(Some(info));
    }

    Ok(McpToolResult::json(&json!({
        "success": true,
        "action": "redo"
    })))
}

fn parse_entity(value: &str) -> Result<EntityId> {
    serde_json::from_value(Value::String(value.to_owned()))
        .with_context(|| format!("invalid entity ID {value}"))
}

fn parse_entities(values: &[String]) -> Result<Vec<EntityId>> {
    values.iter().map(|v| parse_entity(v)).collect()
}

fn parse_point(value: &Value) -> Result<koharu_scene::Point> {
    let x = value
        .get("x")
        .and_then(|v| v.as_f64())
        .context("point is missing numeric 'x'")?;
    let y = value
        .get("y")
        .and_then(|v| v.as_f64())
        .context("point is missing numeric 'y'")?;
    if !x.is_finite() || !y.is_finite() {
        bail!("raster stroke points must be finite");
    }
    Ok(koharu_scene::Point { x, y })
}

fn parse_points(value: &Value) -> Result<Vec<koharu_scene::Point>> {
    let items = value.as_array().context("points must be an array")?;
    if items.is_empty() {
        bail!("points list cannot be empty");
    }
    items.iter().map(parse_point).collect()
}

async fn synchronize(
    handle: &AppHandle<Cef>,
    commit: &koharu_scene::Commit,
    page: Option<EntityId>,
) -> Result<()> {
    let desktop = handle.state::<Desktop>();
    desktop.synchronize(&commit.snapshot, page, commit).await?;
    let canvas = desktop.canvas_state();
    handle.state::<CanvasChannel>().channel.publish(canvas);
    if let Some(info) = handle
        .state::<CurrentProject>()
        .project
        .lock()
        .await
        .as_ref()
        .map(Project::info)
    {
        handle
            .state::<ProjectChannel>()
            .channel
            .publish(Some(info));
    }
    Ok(())
}

fn geometry_from_frame(frame: Frame) -> Result<koharu_scene::Geometry> {
    use koharu_scene::{Geometry as SceneGeometry, Origin, Point as ScenePoint};
    if !frame.x.is_finite()
        || !frame.y.is_finite()
        || !frame.width.is_finite()
        || !frame.height.is_finite()
        || !frame.angle_degrees.is_finite()
        || frame.width <= 0.0
        || frame.height <= 0.0
    {
        bail!("frame must contain finite coordinates and positive dimensions");
    }
    let center_x = f64::from(frame.x + frame.width * 0.5);
    let center_y = f64::from(frame.y + frame.height * 0.5);
    let half_width = f64::from(frame.width) * 0.5;
    let half_height = f64::from(frame.height) * 0.5;
    let (sin, cos) = f64::from(frame.angle_degrees).to_radians().sin_cos();
    Ok(SceneGeometry {
        origin: Origin::User,
        points: [
            (-half_width, -half_height),
            (half_width, -half_height),
            (half_width, half_height),
            (-half_width, half_height),
        ]
        .map(|(x, y)| ScenePoint {
            x: center_x + x * cos - y * sin,
            y: center_y + x * sin + y * cos,
        })
        .into(),
    })
}

async fn select_page(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let page_str: String = serde_json::from_value(
        arguments
            .get("page")
            .cloned()
            .context("missing 'page' argument")?,
    )?;
    let page = parse_entity(&page_str)?;
    let (snapshot, info, selected) = {
        let current = handle.state::<CurrentProject>();
        let mut project = current.project.lock().await;
        let project = project.as_mut().context("no project is open")?;
        project.select_page(page)?;
        let snapshot = project.snapshot();
        let info = project.info();
        let selected = Project::page(&snapshot, page)?;
        (snapshot, info, selected)
    };
    if handle
        .state::<Desktop>()
        .show_page(&snapshot, Some(page))
        .await?
    {
        handle
            .state::<CanvasChannel>()
            .channel
            .publish(handle.state::<Desktop>().canvas_state());
    }
    handle
        .state::<ProjectChannel>()
        .channel
        .publish(Some(info));
    Ok(McpToolResult::json(&selected))
}

async fn close_project(handle: &AppHandle<Cef>) -> Result<McpToolResult> {
    lifecycle::close_current_project(handle).await?;
    Ok(McpToolResult::json(&json!({ "success": true })))
}

async fn delete_project(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let name: String = serde_json::from_value(
        arguments
            .get("name")
            .cloned()
            .context("missing 'name' argument")?,
    )?;
    let active = handle
        .state::<CurrentProject>()
        .project
        .lock()
        .await
        .as_ref()
        .is_some_and(|p| p.name == name);
    if active {
        lifecycle::close_current_project(handle).await?;
    }
    let library = handle.state::<ProjectLibrary>().inner().clone();
    let deleted_name = name.clone();
    tokio::task::spawn_blocking(move || library.delete(&deleted_name))
        .await
        .context("project deletion worker stopped unexpectedly")??;
    Ok(McpToolResult::json(&json!({ "success": true, "deleted_project": name })))
}

async fn export_pages(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    use futures::{StreamExt as _, TryStreamExt as _, stream};
    use image::{ExtendedColorType, ImageEncoder as _};
    use std::sync::Arc;

    let directory_str: String = serde_json::from_value(
        arguments
            .get("directory")
            .cloned()
            .context("missing 'directory' argument")?,
    )?;
    let format_str: String = serde_json::from_value(
        arguments
            .get("format")
            .cloned()
            .context("missing 'format' argument")?,
    )?;
    let pages_arg: Vec<String> = arguments
        .get("pages")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    let directory = PathBuf::from(&directory_str);
    if !directory.is_dir() {
        bail!("directory does not exist: {}", directory.display());
    }
    let snapshot = {
        let current = handle.state::<CurrentProject>();
        let project = current.project.lock().await;
        project.as_ref().context("no project is open")?.snapshot()
    };
    let pages = if pages_arg.is_empty() {
        snapshot.pages().map(|p| p.id()).collect::<Vec<_>>()
    } else {
        parse_entities(&pages_arg)?
    };
    if pages.is_empty() {
        bail!("there are no pages to export");
    }
    let desktop = handle.state::<Desktop>();
    let renderer = desktop.renderer();
    let rasterizer = desktop.rasterizer().await?;
    let jobs = pages
        .into_iter()
        .enumerate()
        .map(|(index, page_id)| {
            let page = snapshot.page(page_id)?.page()?;
            let name = page
                .label
                .trim()
                .trim_end_matches(|c: char| c == '.' || c.is_whitespace());
            let name = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
            let name: String = name
                .chars()
                .map(|c| {
                    if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                        '_'
                    } else {
                        c
                    }
                })
                .collect();
            let stem = format!("{:04}_{}", index + 1, if name.is_empty() { "page" } else { &name });
            Ok::<_, anyhow::Error>((page_id, stem))
        })
        .collect::<Result<Vec<_>>>()?;
    let count = jobs.len();
    let is_psd = match format_str.to_ascii_lowercase().as_str() {
        "png" => false,
        "psd" => true,
        _ => bail!("format must be 'png' or 'psd'"),
    };
    stream::iter(jobs)
        .map(|(page_id, stem)| {
            let renderer = renderer.clone();
            let rasterizer = Arc::clone(&rasterizer);
            let snapshot = snapshot.clone();
            let directory = directory.clone();
            async move {
                let frame = renderer.render(&snapshot, page_id).await?;
                if is_psd {
                    let bytes = koharu_psd::export_page(
                        Arc::clone(&rasterizer),
                        &snapshot,
                        &frame,
                        &koharu_psd::PsdExportOptions::default(),
                    )
                    .await?;
                    tokio::fs::write(directory.join(format!("{stem}.psd")), bytes).await?;
                } else {
                    let raster_frame = frame.raster_frame()?;
                    let raster = tokio::task::spawn_blocking(move || {
                        rasterizer.rasterize(&raster_frame, koharu_rasterizer::RasterOptions::default())
                    })
                    .await
                    .context("rasterizer worker stopped unexpectedly")??;
                    let image = raster.image;
                    let png_stem = stem.clone();
                    tokio::task::spawn_blocking(move || -> Result<()> {
                        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
                        let file =
                            std::fs::File::create(directory.join(format!("{png_stem}.png")))?;
                        PngEncoder::new_with_quality(
                            file,
                            CompressionType::Best,
                            FilterType::Adaptive,
                        )
                        .write_image(
                            image.as_raw(),
                            image.width(),
                            image.height(),
                            ExtendedColorType::Rgba8,
                        )?;
                        Ok(())
                    })
                    .await
                    .context("PNG export worker stopped unexpectedly")??;
                }
                Ok::<_, anyhow::Error>(stem)
            }
        })
        .buffer_unordered(4)
        .try_collect::<Vec<_>>()
        .await?;
    Ok(McpToolResult::json(&json!({
        "success": true,
        "exported_pages_count": count,
        "directory": directory_str,
        "format": format_str
    })))
}

async fn add_point_text(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let x = arguments
        .get("x")
        .and_then(|v| v.as_f64())
        .context("missing numeric 'x'")?;
    let y = arguments
        .get("y")
        .and_then(|v| v.as_f64())
        .context("missing numeric 'y'")?;
    let (commit, page, layer) = {
        let current = handle.state::<CurrentProject>();
        let mut project = current.project.lock().await;
        let project = project.as_mut().context("no project is open")?;
        let active = project.active_page().context("the project has no active page")?;
        let (commit, layer) = project
            .add_point_text(active, crate::commands::canvas::Point { x, y })
            .await?;
        project.record_commit(&commit);
        (commit, project.active_page(), layer)
    };
    synchronize(handle, &commit, page).await?;
    Ok(McpToolResult::json(&json!({
        "success": true,
        "element": layer,
        "revision": commit.revision
    })))
}

async fn commit_paint(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let points = parse_points(arguments.get("points").context("missing 'points'")?)?;
    let diameter = arguments
        .get("diameter")
        .and_then(|v| v.as_f64())
        .context("missing numeric 'diameter'")? as f32;
    let color_vec: Vec<u8> = serde_json::from_value(
        arguments
            .get("color")
            .cloned()
            .context("missing 'color'")?,
    )?;
    if color_vec.len() != 4 {
        bail!("color must be [R,G,B,A]");
    }
    let color = [color_vec[0], color_vec[1], color_vec[2], color_vec[3]];
    let layer = match arguments.get("layer") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(parse_entity(s)?),
        Some(other) => bail!("layer must be a string or null, got {other}"),
    };
    let (commit, page, element) = {
        let current = handle.state::<CurrentProject>();
        let mut project = current.project.lock().await;
        let project = project.as_mut().context("no project is open")?;
        let active = project.active_page().context("the project has no active page")?;
        let (commit, element) = project
            .apply_raster_stroke(active, layer, RasterStrokeMode::Paint, color, diameter, points)
            .await?;
        project.record_commit(&commit);
        (commit, project.active_page(), element)
    };
    synchronize(handle, &commit, page).await?;
    Ok(McpToolResult::json(&json!({
        "success": true,
        "layer": element,
        "revision": commit.revision
    })))
}

async fn commit_erase(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let layer_str: String = serde_json::from_value(
        arguments
            .get("layer")
            .cloned()
            .context("missing 'layer'")?,
    )?;
    let layer = parse_entity(&layer_str)?;
    let points = parse_points(arguments.get("points").context("missing 'points'")?)?;
    let diameter = arguments
        .get("diameter")
        .and_then(|v| v.as_f64())
        .context("missing numeric 'diameter'")? as f32;
    let (commit, page, element) = {
        let current = handle.state::<CurrentProject>();
        let mut project = current.project.lock().await;
        let project = project.as_mut().context("no project is open")?;
        let active = project.active_page().context("the project has no active page")?;
        let (commit, element) = project
            .apply_raster_stroke(active, Some(layer), RasterStrokeMode::Erase, [0; 4], diameter, points)
            .await?;
        project.record_commit(&commit);
        (commit, project.active_page(), element)
    };
    synchronize(handle, &commit, page).await?;
    Ok(McpToolResult::json(&json!({
        "success": true,
        "layer": element,
        "revision": commit.revision
    })))
}

async fn commit_transform(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    let elements = arguments
        .get("elements")
        .and_then(|v| v.as_array())
        .context("missing 'elements' array")?
        .clone();
    if elements.is_empty() {
        bail!("elements list cannot be empty");
    }
    let mut geometries = Vec::with_capacity(elements.len());
    for item in &elements {
        let element_str: String = serde_json::from_value(
            item.get("element").cloned().context("missing 'element'")?,
        )?;
        let element = parse_entity(&element_str)?;
        let frame = Frame {
            x: item.get("x").and_then(|v| v.as_f64()).context("missing 'x'")? as f32,
            y: item.get("y").and_then(|v| v.as_f64()).context("missing 'y'")? as f32,
            width: item.get("width").and_then(|v| v.as_f64()).context("missing 'width'")? as f32,
            height: item.get("height").and_then(|v| v.as_f64()).context("missing 'height'")? as f32,
            angle_degrees: item.get("angle_degrees").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        };
        geometries.push((element, geometry_from_frame(frame)?));
    }
    let (commit, page) = {
        let current = handle.state::<CurrentProject>();
        let mut project = current.project.lock().await;
        let project = project.as_mut().context("no project is open")?;
        let commit = project.set_geometries(geometries).await?;
        project.record_commit(&commit);
        (commit, project.active_page())
    };
    synchronize(handle, &commit, page).await?;
    Ok(McpToolResult::json(&json!({
        "success": true,
        "revision": commit.revision
    })))
}

async fn request_inpaint(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    use std::sync::Arc;

    let page_str: String = serde_json::from_value(
        arguments
            .get("page")
            .cloned()
            .context("missing 'page'")?,
    )?;
    let page = parse_entity(&page_str)?;
    let points_value = arguments.get("points").context("missing 'points'")?.clone();
    let points = parse_points(&points_value)?;
    let diameter = arguments
        .get("diameter")
        .and_then(|v| v.as_f64())
        .context("missing numeric 'diameter'")? as f32;
    if !diameter.is_finite() || diameter <= 0.0 {
        bail!("diameter must be positive");
    }
    let canvas_points: Vec<crate::commands::canvas::Point> = points
        .iter()
        .map(|p| crate::commands::canvas::Point { x: p.x, y: p.y })
        .collect();
    let (width, height) = {
        let current = handle.state::<CurrentProject>();
        let project = current.project.lock().await;
        let project = project.as_ref().context("no project is open")?;
        let snapshot = project.snapshot();
        let value = snapshot.page(page)?.page()?;
        (value.width.round() as u32, value.height.round() as u32)
    };
    let (png, bounds) = tokio::task::spawn_blocking(move || encode_inpaint_mask(width, height, &canvas_points, diameter))
        .await
        .context("inpaint mask worker stopped unexpectedly")??;
    *handle.state::<Processing>().inpainting_mask.lock() = Some(koharu_pipeline::InpaintingMask {
        page,
        png: Arc::from(png),
    });
    let job = crate::commands::processing::process(
        handle.clone(),
        koharu_pipeline::Scope::Region { page, bounds },
        koharu_pipeline::Operation::Only { stage: koharu_pipeline::Stage::Inpainting },
        handle.state::<CurrentProject>(),
        handle.state::<Processing>(),
        handle.state::<JobChannel>(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    Ok(McpToolResult::json(&json!({ "success": true, "job": job })))
}

fn encode_inpaint_mask(
    width: u32,
    height: u32,
    points: &[crate::commands::canvas::Point],
    diameter: f32,
) -> Result<(Vec<u8>, koharu_pipeline::Bounds)> {
    use image::{GrayImage, ImageEncoder as _, codecs::png::PngEncoder};
    if width == 0 || height == 0 {
        bail!("page dimensions must be positive");
    }
    let mut image = GrayImage::new(width, height);
    let radius = f64::from(diameter) * 0.5;
    let mut dirty = None::<[u32; 4]>;
    for (start, end) in points
        .iter()
        .zip(points.iter().skip(1))
        .chain(points.last().map(|point| (point, point)))
    {
        let left = (start.x.min(end.x) - radius - 0.5).floor().max(0.0) as u32;
        let top = (start.y.min(end.y) - radius - 0.5).floor().max(0.0) as u32;
        let right = (start.x.max(end.x) + radius + 0.5).ceil().min(f64::from(width)) as u32;
        let bottom = (start.y.max(end.y) + radius + 0.5).ceil().min(f64::from(height)) as u32;
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length_squared = dx.mul_add(dx, dy * dy);
        for y in top..bottom {
            for x in left..right {
                let px = f64::from(x) + 0.5;
                let py = f64::from(y) + 0.5;
                let progress = if length_squared <= f64::EPSILON {
                    0.0
                } else {
                    (((px - start.x) * dx + (py - start.y) * dy) / length_squared).clamp(0.0, 1.0)
                };
                let nearest_x = start.x + progress * dx;
                let nearest_y = start.y + progress * dy;
                let distance = (px - nearest_x).hypot(py - nearest_y);
                let coverage = ((radius + 0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8;
                if coverage == 0 {
                    continue;
                }
                let pixel = image.get_pixel_mut(x, y);
                pixel.0[0] = pixel.0[0].max(coverage);
                dirty = Some(match dirty {
                    Some([min_x, min_y, max_x, max_y]) => {
                        [min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y)]
                    }
                    None => [x, y, x, y],
                });
            }
        }
    }
    let [left, top, right, bottom] = dirty.context("inpaint stroke does not intersect the page")?;
    let mut png = Vec::new();
    PngEncoder::new(&mut png).write_image(
        image.as_raw(),
        width,
        height,
        image::ExtendedColorType::L8,
    )?;
    Ok((
        png,
        koharu_pipeline::Bounds {
            x: f64::from(left),
            y: f64::from(top),
            width: f64::from(right - left + 1),
            height: f64::from(bottom - top + 1),
        },
    ))
}

async fn stop_processing(handle: &AppHandle<Cef>) -> Result<McpToolResult> {
    let processing = handle.state::<Processing>();
    let count = processing.stops.lock().len();
    for stop in processing.stops.lock().values() {
        stop.stop();
    }
    Ok(McpToolResult::json(&json!({ "success": true, "stopped_jobs": count })))
}

async fn get_fonts(handle: &AppHandle<Cef>) -> Result<McpToolResult> {
    let fonts = handle
        .state::<Desktop>()
        .renderer()
        .available_fonts()
        .await?;
    let names = fonts.into_iter().map(|f| f.name).collect::<Vec<_>>();
    Ok(McpToolResult::json(&json!({ "fonts": names })))
}

async fn get_translation_models() -> Result<McpToolResult> {
    let models = koharu_translator::Translator::models().await?;
    Ok(McpToolResult::json(&models))
}

async fn get_preferences() -> Result<McpToolResult> {
    let preferences = Preferences::load()?;
    Ok(McpToolResult::json(&preferences))
}

async fn get_page_thumbnail(handle: &AppHandle<Cef>, arguments: Value) -> Result<McpToolResult> {
    use koharu_scene::AssetRole;

    let page_str: String = serde_json::from_value(
        arguments
            .get("page")
            .cloned()
            .context("missing 'page'")?,
    )?;
    let page = parse_entity(&page_str)?;
    let (blob_bytes, label) = {
        let current = handle.state::<CurrentProject>();
        let project = current.project.lock().await;
        let project = project.as_ref().context("no project is open")?;
        let snapshot = project.snapshot();
        let label = snapshot.page(page)?.page()?.label;
        let blob = snapshot
            .asset(page, &AssetRole::new("source")?)?
            .with_context(|| format!("page {page} has no source image"))?
            .blob;
        let bytes = snapshot.read_blob(blob).await?;
        (bytes, label)
    };
    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let image = image::load_from_memory(&blob_bytes).context("failed to decode source image")?;
        let image = image.thumbnail(256, 256).to_rgba8();
        let encoder = webp::Encoder::from_rgba(image.as_raw(), image.width(), image.height());
        Ok(encoder.encode(80.0).to_vec())
    })
    .await
    .context("thumbnail worker stopped unexpectedly")??;
    Ok(McpToolResult {
        content: vec![
            McpContent::Image {
                mime_type: "image/webp".to_string(),
                data: STANDARD.encode(bytes),
            },
            McpContent::Text {
                text: format!("Thumbnail for page {label} ({page})"),
            },
        ],
        is_error: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tools() {
        let tools = list_tools();
        assert_eq!(tools.len(), 34);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"inspect_project"));
        assert!(names.contains(&"view_page"));
        assert!(names.contains(&"import_images"));
        assert!(names.contains(&"run_pipeline"));
        assert!(names.contains(&"set_translation"));
        assert!(names.contains(&"set_source_text"));
        assert!(names.contains(&"add_text_box"));
        assert!(names.contains(&"set_typography"));
        assert!(names.contains(&"set_geometry"));
        assert!(names.contains(&"set_visibility"));
        assert!(names.contains(&"delete_elements"));
        assert!(names.contains(&"move_element"));
        assert!(names.contains(&"rename_page"));
        assert!(names.contains(&"move_page"));
        assert!(names.contains(&"delete_pages"));
        assert!(names.contains(&"open_project"));
        assert!(names.contains(&"create_project"));
        assert!(names.contains(&"list_projects"));
        assert!(names.contains(&"undo"));
        assert!(names.contains(&"redo"));
        assert!(names.contains(&"select_page"));
        assert!(names.contains(&"close_project"));
        assert!(names.contains(&"delete_project"));
        assert!(names.contains(&"export_pages"));
        assert!(names.contains(&"add_point_text"));
        assert!(names.contains(&"commit_paint"));
        assert!(names.contains(&"commit_erase"));
        assert!(names.contains(&"commit_transform"));
        assert!(names.contains(&"request_inpaint"));
        assert!(names.contains(&"stop_processing"));
        assert!(names.contains(&"get_fonts"));
        assert!(names.contains(&"get_translation_models"));
        assert!(names.contains(&"get_preferences"));
        assert!(names.contains(&"get_page_thumbnail"));
    }
}
