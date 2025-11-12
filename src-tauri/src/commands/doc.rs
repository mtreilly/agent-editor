//! Document CRUD commands

use crate::db::Db;
use rusqlite::params;
use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DocCreate {
    pub repo_id: String,
    pub slug: String,
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct DocUpdate {
    pub doc_id: String,
    pub body: String,
    pub message: Option<String>,
}

/// Helper function to compute document version hash
fn doc_version_hash(doc_id: &str, body: &str) -> String {
    let body_hash = blake3::hash(body.as_bytes()).to_hex().to_string();
    format!("{doc_id}:{body_hash}")
}

#[tauri::command]
pub async fn docs_create(
    payload: DocCreate,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let mut conn = db.0.lock();
    let doc_id = Uuid::new_v4().to_string();
    let blob_id = Uuid::new_v4().to_string();
    let version_id = Uuid::new_v4().to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO folder(id,repo_id,path,slug) VALUES(?,?,?,?) ON CONFLICT(repo_id,path) DO NOTHING",
        params![Uuid::new_v4().to_string(), payload.repo_id, "", ""],
    )
    .ok();
    tx.execute(
        "INSERT INTO doc(id,repo_id,folder_id,slug,title,size_bytes,line_count) VALUES(?,?,?,?,?,?,?)",
        params![
            doc_id,
            payload.repo_id,
            tx.last_insert_rowid(),
            payload.slug,
            payload.title,
            payload.body.len() as i64,
            payload.body.lines().count() as i64
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)",
        params![blob_id, payload.body.as_bytes(), payload.body.len() as i64],
    )
    .map_err(|e| e.to_string())?;
    let version_hash = doc_version_hash(&doc_id, &payload.body);
    tx.execute(
        "INSERT INTO doc_version(id,doc_id,blob_id,hash) VALUES(?,?,?,?)",
        params![version_id, doc_id, blob_id, version_hash],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE doc SET current_version_id=?1 WHERE id=?2",
        params![version_id, doc_id],
    )
    .map_err(|e| e.to_string())?;
    // FTS update
    tx.execute(
        "INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid,d.title,?1,d.slug,d.repo_id FROM doc d WHERE d.id=?2",
        params![payload.body, doc_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    // update links
    crate::graph::update_links_for_doc(&db.0.lock(), &doc_id, &payload.body)?;
    Ok(serde_json::json!({"doc_id": doc_id}))
}

#[tauri::command]
pub async fn docs_update(
    payload: DocUpdate,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let mut conn = db.0.lock();
    let version_hash = doc_version_hash(&payload.doc_id, &payload.body);
    // Check if same as current
    let unchanged: bool = conn
        .query_row(
            "SELECT v.hash FROM doc d JOIN doc_version v ON v.id=d.current_version_id WHERE d.id=?1",
            params![&payload.doc_id],
            |r| r.get::<_, String>(0),
        )
        .map(|h| h == version_hash)
        .unwrap_or(false);
    if unchanged {
        let cur: String = conn
            .query_row(
                "SELECT current_version_id FROM doc WHERE id=?1",
                params![&payload.doc_id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        drop(conn);
        return Ok(serde_json::json!({"version_id": cur, "skipped": true}));
    }
    let version_id = Uuid::new_v4().to_string();
    let blob_id = Uuid::new_v4().to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)",
        params![blob_id, payload.body.as_bytes(), payload.body.len() as i64],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO doc_version(id,doc_id,blob_id,hash,message) VALUES(?,?,?,?,?)",
        params![
            version_id,
            payload.doc_id,
            blob_id,
            version_hash,
            payload.message.unwrap_or_default()
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE doc SET current_version_id=?1, size_bytes=?2, line_count=?3, updated_at=datetime('now') WHERE id=?4",
        params![
            version_id,
            payload.body.len() as i64,
            payload.body.lines().count() as i64,
            payload.doc_id
        ],
    )
    .map_err(|e| e.to_string())?;
    // FTS update: delete+insert
    tx.execute(
        "INSERT INTO doc_fts(doc_fts,rowid) VALUES('delete',(SELECT rowid FROM doc WHERE id=?1))",
        params![payload.doc_id],
    )
    .ok();
    tx.execute(
        "INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid,d.title,?1,d.slug,d.repo_id FROM doc d WHERE d.id=?2",
        params![payload.body, payload.doc_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    // release connection lock before link update to avoid deadlock
    drop(conn);
    // update links
    crate::graph::update_links_for_doc(&db.0.lock(), &payload.doc_id, &payload.body)?;
    Ok(serde_json::json!({"version_id": version_id}))
}

#[tauri::command]
pub async fn docs_get(
    doc_id: String,
    content: Option<bool>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare("SELECT id,repo_id,slug,title,current_version_id FROM doc WHERE id=?1 OR slug=?1 LIMIT 1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![doc_id]).map_err(|e| e.to_string())?;
    if let Some(r) = rows.next().map_err(|e| e.to_string())? {
        let id: String = r.get(0).unwrap_or_default();
        let include_body = content.unwrap_or(false);
        let mut out = serde_json::json!({
            "id": id,
            "repo_id": r.get::<_, String>(1).unwrap_or_default(),
            "slug": r.get::<_, String>(2).unwrap_or_default(),
            "title": r.get::<_, String>(3).unwrap_or_default(),
            "current_version_id": r.get::<_, String>(4).unwrap_or_default(),
        });
        if include_body {
            let body: Option<String> = conn
                .query_row(
                    "SELECT body FROM doc_fts WHERE rowid=(SELECT rowid FROM doc WHERE id=?1)",
                    params![&id],
                    |rr| rr.get(0),
                )
                .ok();
            out["body"] = body
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null);
        }
        return Ok(out);
    }
    Err("not_found".into())
}

#[tauri::command]
pub async fn docs_delete(
    doc_id: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "UPDATE doc SET is_deleted=1 WHERE id=?1 OR slug=?1",
            params![doc_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"deleted": n>0}))
}
