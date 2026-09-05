//! Koharu's Tauri-managed application state, commands, and lifecycle.

mod app;
mod commands;
pub(crate) mod mcp;

pub use app::run;
pub use commands::bindings;
