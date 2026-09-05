use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context as _, Result, bail};
use koharu_agent::{Control, Host, ToolCall};
use koharu_desktop::Desktop;
use koharu_scene::{AssetInput, AssetMetadata, AssetRole, At, PageDraft};
use serde_json::{Value, json};
use tauri::{AppHandle, Cef, Manager as _};
use walkdir::WalkDir;

use super::types::{McpContent, McpToolDefinition, McpToolResult};
use crate::commands::{
    ChannelExt as _,
    agent::host::KoharuHost,
    canvas::CanvasChannel,
    import::{self, Format},
    lifecycle,
    project::{CurrentProject, ProjectLibrary},
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
        ]
    })
}

pub async fn execute_tool(
    handle: &AppHandle<Cef>,
    name: &str,
    arguments: Value,
) -> Result<McpToolResult> {
    match name {
        "import_images" => import_images(handle, arguments).await,
        "open_project" => open_project(handle, arguments).await,
        "create_project" => create_project(handle, arguments).await,
        "list_projects" => list_projects(handle).await,
        "undo" => undo(handle).await,
        "redo" => redo(handle).await,
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
    let (commit, page) = {
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
        (commit, project.active_page())
    };

    let desktop = handle.state::<Desktop>();
    desktop.synchronize(&commit.snapshot, page, &commit).await?;
    let canvas = desktop.canvas_state();
    handle.state::<CanvasChannel>().channel.publish(canvas);

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

    Ok(McpToolResult::json(&json!({
        "success": true,
        "action": "redo"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tools() {
        let tools = list_tools();
        assert_eq!(tools.len(), 19);
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
        assert!(names.contains(&"rename_page"));
        assert!(names.contains(&"move_page"));
        assert!(names.contains(&"delete_pages"));
        assert!(names.contains(&"open_project"));
        assert!(names.contains(&"create_project"));
        assert!(names.contains(&"list_projects"));
        assert!(names.contains(&"undo"));
        assert!(names.contains(&"redo"));
    }
}
