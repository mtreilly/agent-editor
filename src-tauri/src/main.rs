#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod commands;
mod api;
mod scan;
mod graph;
mod secrets;

use std::path::PathBuf;
use tauri::Manager;

fn main() {
    let ctx = tauri::generate_context!();
    // Dev-friendly DB location; packaging may override via AE_DB
    let db_path = std::env::var("AE_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let p = PathBuf::from(".dev/agent-editor.db");
            if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
            p
        });
    let db_state = std::sync::Arc::new(db::open_db(&db_path).expect("open db"));

    tauri::Builder::default()
        .manage(db_state)
        .setup(|app| {
            let db = app.state::<std::sync::Arc<db::Db>>().inner().clone();
            tauri::async_runtime::spawn(async move { let _ = api::start_api(db, 35678).await; });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::repos_add,
            commands::repos_list,
            commands::repos_info,
            commands::repos_remove,
            commands::scan_repo,
            commands::docs_create,
            commands::docs_update,
            commands::docs_get,
            commands::docs_delete,
            commands::search,
            commands::ai_run,
            commands::ai_providers_list,
            commands::ai_providers_enable,
            commands::ai_providers_disable,
            commands::ai_provider_key_set,
            commands::ai_provider_key_get,
            commands::plugins_list,
            commands::plugins_info,
            commands::plugins_enable,
            commands::plugins_disable,
            commands::plugins_remove,
            commands::plugins_upsert,
            commands::plugins_call_core,
            commands::plugins_spawn_core,
            commands::plugins_shutdown_core,
            commands::anchors_upsert,
            commands::anchors_list,
            commands::anchors_delete,
            api::serve_api_start,
        ])
        .run(ctx)
        .expect("error while running tauri application");
}
