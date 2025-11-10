// Minimal headless JSON-RPC server to run without Tauri window.
// Uses same DB schema and API as the desktop app.

#![allow(clippy::all)]

#[path = "../db.rs"]
mod db;
#[path = "../api.rs"]
mod api;
#[path = "../commands.rs"]
mod commands;
#[path = "../scan/mod.rs"]
mod scan;
#[path = "../graph/mod.rs"]
mod graph;

use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_path = env::var("AE_DB").map(PathBuf::from).unwrap_or_else(|_| {
        let p = PathBuf::from(".dev/agent-editor.db");
        if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
        p
    });
    let port: u16 = env::var("AE_RPC_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(35678);
    let db = std::sync::Arc::new(db::open_db(&db_path).expect("open_db"));
    eprintln!("[rpc_sidecar] DB: {}  Port: {}", db_path.display(), port);
    api::start_api(db, port).await.expect("start_api");
    Ok(())
}
