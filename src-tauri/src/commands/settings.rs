//! Application settings commands

use crate::db::Db;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub async fn app_settings_get(
    key: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let val: Option<String> = conn
        .query_row(
            "SELECT value FROM app_setting WHERE key=?1",
            params![key],
            |r| r.get(0),
        )
        .ok();
    Ok(serde_json::json!({
        "value": val
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .unwrap_or(serde_json::json!(null))
    }))
}

#[tauri::command]
pub async fn app_settings_set(
    key: String,
    value: serde_json::Value,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "INSERT INTO app_setting(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')",
            params![key, value.to_string()],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n > 0}))
}
