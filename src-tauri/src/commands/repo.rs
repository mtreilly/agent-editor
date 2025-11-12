//! Repository management commands

use crate::db::Db;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn repos_add(
    path: String,
    name: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });
    let res = {
        let conn = db.0.lock();
        let mut stmt = conn
            .prepare("INSERT OR IGNORE INTO repo(id,name,path,settings) VALUES(?,?,?,json('{}'))")
            .map_err(|e| e.to_string())?;
        stmt.execute(params![id, name, path])
            .map_err(|e| e.to_string())?
    };
    let _ = (include, exclude, res); // reserved for future
    Ok(serde_json::json!({"repo_id": id}))
}

#[tauri::command]
pub async fn repos_list(db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare("SELECT id,name,path FROM repo ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, String>(0)?,
                "name": r.get::<_, String>(1)?,
                "path": r.get::<_, String>(2)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?)
    }
    Ok(out)
}

#[tauri::command]
pub async fn repos_info(
    id_or_name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id,name,path,settings,created_at,updated_at FROM repo WHERE id=?1 OR name=?1",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![id_or_name]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let settings: Option<String> = row.get(3).ok();
        return Ok(serde_json::json!({
            "id": row.get::<_, String>(0).unwrap_or_default(),
            "name": row.get::<_, String>(1).unwrap_or_default(),
            "path": row.get::<_, String>(2).unwrap_or_default(),
            "settings": settings.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::json!({})),
            "created_at": row.get::<_, String>(4).unwrap_or_default(),
            "updated_at": row.get::<_, String>(5).unwrap_or_default(),
        }));
    }
    Err("not_found".into())
}

#[tauri::command]
pub async fn repos_remove(
    id_or_name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "DELETE FROM repo WHERE id=?1 OR name=?1",
            params![id_or_name],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"removed": n>0}))
}

#[tauri::command]
pub async fn repos_set_default_provider(
    id_or_name: String,
    provider: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "UPDATE repo SET settings=json_set(COALESCE(settings,json('{}')),'$.default_provider',?2), updated_at=datetime('now') WHERE id=?1 OR name=?1",
            params![id_or_name, provider],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}
