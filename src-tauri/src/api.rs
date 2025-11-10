use std::{net::SocketAddr, sync::Arc};

use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::db::Db;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;
use tauri::Emitter;

#[derive(Deserialize)]
struct RpcReq<T=serde_json::Value> { jsonrpc: String, id: String, method: String, params: Option<T> }

#[derive(Serialize)]
struct RpcRes { jsonrpc: String, id: String, #[serde(skip_serializing_if = "Option::is_none")] result: Option<serde_json::Value>, #[serde(skip_serializing_if = "Option::is_none")] error: Option<RpcErr> }

#[derive(Serialize)]
struct RpcErr { code: i32, message: String }

pub async fn start_api(db: Arc<Db>, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new().route("/rpc", post(handler)).with_state(db);
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn handler(
    axum::extract::State(db): axum::extract::State<Arc<Db>>,
    Json(req): Json<RpcReq>,
) -> Json<RpcRes> {
    let id = req.id.clone();
    let result = route(req, db).await;
    match result {
        Ok(v) => Json(RpcRes { jsonrpc: "2.0".into(), id, result: Some(v), error: None }),
        Err(e) => Json(RpcRes { jsonrpc: "2.0".into(), id, result: None, error: Some(RpcErr { code: -32000, message: e }) }),
    }
}

async fn route(req: RpcReq, db: Arc<Db>) -> Result<serde_json::Value, String> {
    match req.method.as_str() {
        "repos_add" => {
            #[derive(Deserialize)] struct P { path: String, name: Option<String>, include: Option<Vec<String>>, exclude: Option<Vec<String>> }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let id = Uuid::new_v4().to_string();
            let name = p.name.unwrap_or_else(|| std::path::Path::new(&p.path).file_name().unwrap_or_default().to_string_lossy().to_string());
            let mut conn = db.0.lock();
            conn.execute("INSERT OR IGNORE INTO repo(id,name,path,settings) VALUES(?,?,?,json('{}'))", params![id, name, p.path]).map_err(|e| e.to_string())?;
            let _ = (p.include, p.exclude);
            Ok(serde_json::json!({"repo_id": id}))
        }
        "repos_list" => {
            let mut conn = db.0.lock();
            let mut stmt = conn.prepare("SELECT id,name,path FROM repo ORDER BY created_at DESC").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| Ok(serde_json::json!({"id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "path": r.get::<_, String>(2)?}))).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows { out.push(r.map_err(|e| e.to_string())?) }
            Ok(serde_json::json!(out))
        }
        "repos_info" => {
            #[derive(Deserialize)] struct P { id_or_name: String }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let mut conn = db.0.lock();
            let mut stmt = conn.prepare("SELECT id,name,path,settings,created_at,updated_at FROM repo WHERE id=?1 OR name=?1")
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![p.id_or_name]).map_err(|e| e.to_string())?;
            if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                let settings: Option<String> = row.get(3).ok();
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0).unwrap_or_default(),
                    "name": row.get::<_, String>(1).unwrap_or_default(),
                    "path": row.get::<_, String>(2).unwrap_or_default(),
                    "settings": settings.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::json!({})),
                    "created_at": row.get::<_, String>(4).unwrap_or_default(),
                    "updated_at": row.get::<_, String>(5).unwrap_or_default(),
                }))
            } else { Err("not_found".into()) }
        }
        "repos_remove" => {
            #[derive(Deserialize)] struct P { id_or_name: String }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let mut conn = db.0.lock();
            let n = conn.execute("DELETE FROM repo WHERE id=?1 OR name=?1", params![p.id_or_name]).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({"removed": n>0}))
        }
        "scan_repo" => {
            #[derive(Deserialize)] struct P { repo_path: String, filters: Option<serde_json::Value>, watch: Option<bool>, debounce: Option<u64> }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let job_id = Uuid::new_v4().to_string();
            let mut conn = db.0.lock();
            let repo_id: Option<String> = conn.query_row("SELECT id FROM repo WHERE path=?1 OR name=?1", params![p.repo_path], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
            let repo_id = repo_id.unwrap_or_else(|| { let id = Uuid::new_v4().to_string(); conn.execute("INSERT OR IGNORE INTO repo(id,name,path) VALUES(?,?,?)", params![id.clone(), &p.repo_path, &p.repo_path]).ok(); id });
            conn.execute("INSERT INTO scan_job(id,repo_id,status,stats) VALUES(?,?,'queued',json('{}'))", params![job_id, repo_id]).map_err(|e| e.to_string())?;
            let _ = (p.filters, p.watch, p.debounce);
            Ok(serde_json::json!({"job_id": job_id, "files_scanned": 0, "docs_added": 0, "errors": 0}))
        }
        "docs_create" => {
            #[derive(Deserialize)] struct P { repo_id: String, slug: String, title: String, body: String }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let mut conn = db.0.lock();
            let doc_id = Uuid::new_v4().to_string();
            let blob_id = Uuid::new_v4().to_string();
            let version_id = Uuid::new_v4().to_string();
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO doc(id,repo_id,folder_id,slug,title,size_bytes,line_count) VALUES(?,?,?,?,?,?,?)",
                params![doc_id, p.repo_id, tx.last_insert_rowid(), p.slug, p.title, p.body.len() as i64, p.body.lines().count() as i64]).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)", params![blob_id, p.body.as_bytes(), p.body.len() as i64]).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO doc_version(id,doc_id,blob_id,hash) VALUES(?,?,?,?)", params![version_id, doc_id, blob_id, version_id]).map_err(|e| e.to_string())?;
            tx.execute("UPDATE doc SET current_version_id=?1 WHERE id=?2", params![version_id, doc_id]).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid,d.title,?1,d.slug,d.repo_id FROM doc d WHERE d.id=?2", params![p.body, doc_id]).map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(serde_json::json!({"doc_id": doc_id}))
        }
        "docs_update" => {
            #[derive(Deserialize)] struct P { doc_id: String, body: String, message: Option<String> }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let mut conn = db.0.lock();
            let version_id = Uuid::new_v4().to_string();
            let blob_id = Uuid::new_v4().to_string();
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)", params![blob_id, p.body.as_bytes(), p.body.len() as i64]).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO doc_version(id,doc_id,blob_id,hash,message) VALUES(?,?,?,?,?)", params![version_id, p.doc_id, blob_id, version_id, p.message.unwrap_or_default()]).map_err(|e| e.to_string())?;
            tx.execute("UPDATE doc SET current_version_id=?1, size_bytes=?2, line_count=?3, updated_at=datetime('now') WHERE id=?4", params![version_id, p.body.len() as i64, p.body.lines().count() as i64, p.doc_id]).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO doc_fts(doc_fts,rowid) VALUES('delete',(SELECT rowid FROM doc WHERE id=?1))", params![p.doc_id]).ok();
            tx.execute("INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid,d.title,?1,d.slug,d.repo_id FROM doc d WHERE d.id=?2", params![p.body, p.doc_id]).map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(serde_json::json!({"version_id": version_id}))
        }
        "docs_get" => {
            #[derive(Deserialize)] struct P { doc_id: String, content: Option<bool> }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let conn = db.0.lock();
            let mut stmt = conn.prepare("SELECT id,repo_id,slug,title,current_version_id FROM doc WHERE id=?1 OR slug=?1 LIMIT 1").map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![p.doc_id]).map_err(|e| e.to_string())?;
            if let Some(r) = rows.next().map_err(|e| e.to_string())? {
                let id: String = r.get(0).unwrap_or_default();
                let mut out = serde_json::json!({
                    "id": id,
                    "repo_id": r.get::<_, String>(1).unwrap_or_default(),
                    "slug": r.get::<_, String>(2).unwrap_or_default(),
                    "title": r.get::<_, String>(3).unwrap_or_default(),
                    "current_version_id": r.get::<_, String>(4).unwrap_or_default(),
                });
                if p.content.unwrap_or(false) {
                    let body: Option<String> = conn.query_row("SELECT body FROM doc_fts WHERE rowid=(SELECT rowid FROM doc WHERE id=?1)", params![&id], |rr| rr.get(0)).ok();
                    out["body"] = body.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
                }
                Ok(out)
            } else { Err("not_found".into()) }
        }
        "docs_delete" => {
            #[derive(Deserialize)] struct P { doc_id: String }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let conn = db.0.lock();
            let n = conn.execute("UPDATE doc SET is_deleted=1 WHERE id=?1 OR slug=?1", params![p.doc_id]).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({"deleted": n>0}))
        }
        "search" => {
            #[derive(Deserialize)] struct P { repo_id: Option<String>, query: String, limit: Option<i64>, offset: Option<i64> }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let conn = db.0.lock();
            let lim = p.limit.unwrap_or(50);
            let off = p.offset.unwrap_or(0);
            let mut sql = String::from("SELECT d.id, d.slug, bm25(doc_fts, 1.2, 0.75) as rank, snippet(doc_fts,1,'<b>','</b>','…',8) as title_snip, snippet(doc_fts,2,'<b>','</b>','…',8) as body_snip FROM doc_fts JOIN doc d ON d.rowid=doc_fts.rowid WHERE doc_fts MATCH ?1");
            if p.repo_id.is_some() { sql.push_str(" AND d.repo_id=?2"); }
            sql.push_str(" ORDER BY rank ASC, d.updated_at DESC LIMIT ?3 OFFSET ?4");
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            if let Some(repo) = p.repo_id {
                let rows = stmt.query_map(params![p.query, repo, lim, off], |r| Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "slug": r.get::<_, String>(1)?,
                    "rank": r.get::<_, f64>(2).unwrap_or(0.0),
                    "title_snip": r.get::<_, String>(3).unwrap_or_default(),
                    "body_snip": r.get::<_, String>(4).unwrap_or_default()
                }))).map_err(|e| e.to_string())?;
                for r in rows { out.push(r.map_err(|e| e.to_string())?) }
            } else {
                let rows = stmt.query_map(params![p.query, lim, off], |r| Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "slug": r.get::<_, String>(1)?,
                    "rank": r.get::<_, f64>(2).unwrap_or(0.0),
                    "title_snip": r.get::<_, String>(3).unwrap_or_default(),
                    "body_snip": r.get::<_, String>(4).unwrap_or_default()
                }))).map_err(|e| e.to_string())?;
                for r in rows { out.push(r.map_err(|e| e.to_string())?) }
            }
            Ok(serde_json::json!(out))
        }
        "graph_backlinks" => {
            #[derive(Deserialize)] struct P { doc_id: String }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let conn = db.0.lock();
            let mut stmt = conn.prepare("SELECT d.id, d.slug, d.title FROM link l JOIN doc d ON d.id = l.from_doc_id WHERE l.to_doc_id = ?1 ORDER BY d.updated_at DESC").map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![p.doc_id], |r| Ok(serde_json::json!({"id": r.get::<_, String>(0)?, "slug": r.get::<_, String>(1)?, "title": r.get::<_, String>(2)?}))).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows { out.push(r.map_err(|e| e.to_string())?) }
            Ok(serde_json::json!(out))
        }
        "graph_neighbors" => {
            #[derive(Deserialize)] struct P { doc_id: String, depth: Option<u8> }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let conn = db.0.lock();
            let mut stmt = conn.prepare("SELECT DISTINCT d.id, d.slug, d.title FROM (SELECT l2.from_doc_id AS neighbor_id FROM link l JOIN link l2 ON l.to_doc_id = l2.to_doc_id WHERE l.from_doc_id = ?1 AND l2.from_doc_id != ?1 UNION SELECT to_doc_id FROM link WHERE from_doc_id = ?1 AND to_doc_id IS NOT NULL) n JOIN doc d ON d.id = n.neighbor_id").map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![p.doc_id], |r| Ok(serde_json::json!({"id": r.get::<_, String>(0)?, "slug": r.get::<_, String>(1)?, "title": r.get::<_, String>(2)?}))).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for r in rows { out.push(r.map_err(|e| e.to_string())?) }
            Ok(serde_json::json!(out))
        }
        "ai_run" => {
            #[derive(Deserialize)] struct P { provider: String, doc_id: String, anchor_id: Option<String>, prompt: String }
            let p: P = serde_json::from_value(req.params.unwrap_or_default()).map_err(|e| e.to_string())?;
            let res = crate::commands::AiRunRequest { provider: p.provider, doc_id: p.doc_id, anchor_id: p.anchor_id, line: None, prompt: p.prompt };
            crate::commands::ai_run_core(&db, res)
        }
        m => Err(format!("unknown method: {}", m)),
    }
}

#[tauri::command]
pub async fn serve_api_start(port: Option<u16>, db: tauri::State<'_, Arc<Db>>, app: tauri::AppHandle) -> Result<(), String> {
    let port = port.unwrap_or(35678);
    let db = db.inner().clone();
    tauri::async_runtime::spawn(async move {
        let _ = start_api(db, port).await;
    });
    let _ = app.emit("serve_api_started", serde_json::json!({"port": port}));
    Ok(())
}
