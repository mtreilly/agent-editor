#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod commands;
mod api;
mod scan;
mod graph;

use std::path::PathBuf;
use tauri::Manager;

fn main() {
    let ctx = tauri::generate_context!();
    let app_dir: PathBuf = ctx
        .path()
        .app_data_dir()
        .expect("app data dir");
    let db_path = app_dir.join("agent-editor.db");
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
            commands::anchors_upsert,
            commands::anchors_list,
            commands::anchors_delete,
            api::serve_api_start,
        ])
        .run(ctx)
        .expect("error while running tauri application");
}
