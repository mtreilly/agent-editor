//! Document anchor commands

use crate::db::Db;
use rusqlite::params;
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

#[derive(Serialize)]
pub struct AnchorItem {
    pub id: String,
    pub line: i64,
    pub created_at: String,
}

#[tauri::command]
pub async fn anchors_upsert(
    doc_id: String,
    anchor_id: String,
    line: i64,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let id = Uuid::new_v4().to_string();
    let meta = serde_json::json!({"doc_id": doc_id, "line": line});
    conn.execute(
        "INSERT INTO provenance(id,entity_type,entity_id,source,meta) VALUES(?, 'anchor', ?, 'ui', ?)",
        params![id, anchor_id, meta.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
pub async fn anchors_list(
    doc_id: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<AnchorItem>, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare(
            "SELECT entity_id, COALESCE(json_extract(meta,'$.line'), 0), created_at \
         FROM provenance WHERE entity_type='anchor' AND json_extract(meta,'$.doc_id')=?1 \
         ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![doc_id], |r| {
            Ok(AnchorItem {
                id: r.get(0)?,
                line: r.get(1)?,
                created_at: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?)
    }
    Ok(out)
}

#[tauri::command]
pub async fn anchors_delete(
    anchor_id: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "DELETE FROM provenance WHERE entity_type='anchor' AND entity_id=?1",
            params![anchor_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"deleted": n>0}))
}
