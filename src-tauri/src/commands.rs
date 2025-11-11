use crate::{db::Db, scan};
use crate::secrets;
use crate::ai;
use tauri::Emitter;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use std::collections::HashMap;
use std::process::{Child, ChildStdin, ChildStdout, Command as OsCommand, Stdio};
use std::sync::{Mutex, OnceLock};
use std::path::{Path, PathBuf};
use std::io::{Write, BufRead, BufReader};

#[derive(Deserialize)]
pub struct ScanFilters { pub include: Option<Vec<String>>, pub exclude: Option<Vec<String>> }

#[derive(Serialize)]
pub struct ScanJobReport { pub job_id: String, pub files_scanned: i64, pub docs_added: i64, pub errors: i64 }

#[tauri::command]
pub async fn repos_add(path: String, name: Option<String>, include: Option<Vec<String>>, exclude: Option<Vec<String>>, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let id = Uuid::new_v4().to_string();
    let name = name.unwrap_or_else(|| std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy().to_string());
    let res = {
        let conn = db.0.lock();
        let mut stmt = conn.prepare("INSERT OR IGNORE INTO repo(id,name,path,settings) VALUES(?,?,?,json('{}'))")
            .map_err(|e| e.to_string())?;
        stmt.execute(params![id, name, path]).map_err(|e| e.to_string())?
    };
    let _ = (include, exclude, res); // reserved for future
    Ok(serde_json::json!({"repo_id": id}))
}

#[tauri::command]
pub async fn repos_list(db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare("SELECT id,name,path FROM repo ORDER BY created_at DESC").map_err(|e| e.to_string())?;
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
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

#[tauri::command]
pub async fn repos_info(id_or_name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare("SELECT id,name,path,settings,created_at,updated_at FROM repo WHERE id=?1 OR name=?1")
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
pub async fn repos_remove(id_or_name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("DELETE FROM repo WHERE id=?1 OR name=?1", params![id_or_name]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"removed": n>0}))
}

#[tauri::command]
pub async fn repos_set_default_provider(id_or_name: String, provider: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute(
        "UPDATE repo SET settings=json_set(COALESCE(settings,json('{}')),'$.default_provider',?2), updated_at=datetime('now') WHERE id=?1 OR name=?1",
        params![id_or_name, provider],
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn app_settings_get(key: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let val: Option<String> = conn.query_row("SELECT value FROM app_setting WHERE key=?1", params![key], |r| r.get(0)).ok();
    Ok(serde_json::json!({"value": val.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).unwrap_or(serde_json::json!(null))}))
}

#[tauri::command]
pub async fn app_settings_set(key: String, value: serde_json::Value, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute(
        "INSERT INTO app_setting(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')",
        params![key, value.to_string()],
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn scan_repo(repo_path: String, filters: Option<ScanFilters>, watch: Option<bool>, debounce: Option<u64>, db: State<'_, std::sync::Arc<Db>>, app: tauri::AppHandle) -> Result<ScanJobReport, String> {
    let job_id = Uuid::new_v4().to_string();
    let repo_id = {
        let conn = db.0.lock();
        let mut stmt = conn.prepare("SELECT id FROM repo WHERE path=?1 OR name=?1").map_err(|e| e.to_string())?;
        let id: Option<String> = stmt.query_row(params![repo_path], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        id.unwrap_or_else(|| {
            let id = Uuid::new_v4().to_string();
            conn.execute("INSERT OR IGNORE INTO repo(id,name,path) VALUES(?,?,?)", params![id, &repo_path, &repo_path]).ok();
            id
        })
    };
    let conn = db.0.lock();
    conn.execute("INSERT INTO scan_job(id,repo_id,status,stats) VALUES(?,?,'running',json('{}'))", params![job_id, repo_id]).map_err(|e| e.to_string())?;
    drop(conn);
    let include = filters.as_ref().and_then(|f| f.include.clone()).unwrap_or_default();
    let exclude = filters.as_ref().and_then(|f| f.exclude.clone()).unwrap_or_default();
    let stats = scan::scan_once(&db, &repo_path, &include, &exclude)?;
    let conn2 = db.0.lock();
    conn2.execute("UPDATE scan_job SET status='success', stats=?2, finished_at=datetime('now') WHERE id=?1", params![job_id, serde_json::to_string(&serde_json::json!({"files_scanned": stats.files_scanned, "docs_added": stats.docs_added, "errors": stats.errors})).unwrap()]).map_err(|e| e.to_string())?;
    if watch.unwrap_or(false) {
        let _ = app.emit("progress.scan", serde_json::json!({"event": "watch-start", "path": repo_path}));
        let _ = scan::watch_repo(db.inner().clone(), repo_path.clone(), include, exclude, debounce.unwrap_or(200), app);
    }
    Ok(ScanJobReport { job_id, files_scanned: stats.files_scanned, docs_added: stats.docs_added, errors: stats.errors })
}

#[derive(Deserialize)]
pub struct DocCreate { pub repo_id: String, pub slug: String, pub title: String, pub body: String }

#[tauri::command]
pub async fn docs_create(payload: DocCreate, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let mut conn = db.0.lock();
    let doc_id = Uuid::new_v4().to_string();
    let blob_id = Uuid::new_v4().to_string();
    let version_id = Uuid::new_v4().to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO folder(id,repo_id,path,slug) VALUES(?,?,?,?) ON CONFLICT(repo_id,path) DO NOTHING", params![Uuid::new_v4().to_string(), payload.repo_id, "", ""]).ok();
    tx.execute("INSERT INTO doc(id,repo_id,folder_id,slug,title,size_bytes,line_count) VALUES(?,?,?,?,?,?,?)",
        params![doc_id, payload.repo_id, tx.last_insert_rowid(), payload.slug, payload.title, payload.body.len() as i64, payload.body.lines().count() as i64])
        .map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)", params![blob_id, payload.body.as_bytes(), payload.body.len() as i64]).map_err(|e| e.to_string())?;
    let body_hash = blake3::hash(payload.body.as_bytes()).to_hex().to_string();
    let version_hash = format!("{}:{}", doc_id, body_hash);
    tx.execute("INSERT INTO doc_version(id,doc_id,blob_id,hash) VALUES(?,?,?,?)", params![version_id, doc_id, blob_id, version_hash]).map_err(|e| e.to_string())?;
    tx.execute("UPDATE doc SET current_version_id=?1 WHERE id=?2", params![version_id, doc_id]).map_err(|e| e.to_string())?;
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

#[derive(Deserialize)]
pub struct DocUpdate { pub doc_id: String, pub body: String, pub message: Option<String> }

#[tauri::command]
pub async fn docs_update(payload: DocUpdate, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let mut conn = db.0.lock();
    let body_hash = blake3::hash(payload.body.as_bytes()).to_hex().to_string();
    let version_hash = format!("{}:{}", &payload.doc_id, body_hash);
    // Check if same as current
    let unchanged: bool = conn.query_row(
        "SELECT v.hash FROM doc d JOIN doc_version v ON v.id=d.current_version_id WHERE d.id=?1",
        params![&payload.doc_id],
        |r| r.get::<_, String>(0),
    ).map(|h| h == version_hash).unwrap_or(false);
    if unchanged {
        let cur: String = conn.query_row("SELECT current_version_id FROM doc WHERE id=?1", params![&payload.doc_id], |r| r.get(0)).unwrap_or_default();
        drop(conn);
        return Ok(serde_json::json!({"version_id": cur, "skipped": true}));
    }
    let version_id = Uuid::new_v4().to_string();
    let blob_id = Uuid::new_v4().to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)", params![blob_id, payload.body.as_bytes(), payload.body.len() as i64]).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO doc_version(id,doc_id,blob_id,hash,message) VALUES(?,?,?,?,?)", params![version_id, payload.doc_id, blob_id, version_hash, payload.message.unwrap_or_default()]).map_err(|e| e.to_string())?;
    tx.execute("UPDATE doc SET current_version_id=?1, size_bytes=?2, line_count=?3, updated_at=datetime('now') WHERE id=?4",
        params![version_id, payload.body.len() as i64, payload.body.lines().count() as i64, payload.doc_id]).map_err(|e| e.to_string())?;
    // FTS update: delete+insert
    tx.execute("INSERT INTO doc_fts(doc_fts,rowid) VALUES('delete',(SELECT rowid FROM doc WHERE id=?1))", params![payload.doc_id]).ok();
    tx.execute("INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid,d.title,?1,d.slug,d.repo_id FROM doc d WHERE d.id=?2",
        params![payload.body, payload.doc_id]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    // release connection lock before link update to avoid deadlock
    drop(conn);
    // update links
    crate::graph::update_links_for_doc(&db.0.lock(), &payload.doc_id, &payload.body)?;
    Ok(serde_json::json!({"version_id": version_id}))
}

#[tauri::command]
pub async fn docs_get(doc_id: String, content: Option<bool>, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare("SELECT id,repo_id,slug,title,current_version_id FROM doc WHERE id=?1 OR slug=?1 LIMIT 1").map_err(|e| e.to_string())?;
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
            let body: Option<String> = conn.query_row("SELECT body FROM doc_fts WHERE rowid=(SELECT rowid FROM doc WHERE id=?1)", params![&id], |rr| rr.get(0)).ok();
            out["body"] = body.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
        }
        return Ok(out);
    }
    Err("not_found".into())
}

#[tauri::command]
pub async fn docs_delete(doc_id: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("UPDATE doc SET is_deleted=1 WHERE id=?1 OR slug=?1", params![doc_id]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"deleted": n>0}))
}

#[derive(Serialize)]
pub struct SearchHit { pub id: String, pub slug: String, pub title_snip: String, pub body_snip: String, pub rank: f64 }

#[tauri::command]
pub async fn search(repo_id: Option<String>, query: String, limit: Option<i64>, offset: Option<i64>, db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<SearchHit>, String> {
    let conn = db.0.lock();
    let lim = limit.unwrap_or(50);
    let off = offset.unwrap_or(0);
    let sql = String::from(
        "SELECT d.id, d.slug, bm25(doc_fts, 1.2, 0.75) as rank, \
         snippet(doc_fts,1,'<b>','</b>','…',8) as title_snip, \
         snippet(doc_fts,2,'<b>','</b>','…',8) as body_snip \
         FROM doc_fts JOIN doc d ON d.rowid=doc_fts.rowid \
         WHERE doc_fts MATCH ?1 AND (?2 IS NULL OR d.repo_id = ?2) \
         ORDER BY rank ASC, d.updated_at DESC LIMIT ?3 OFFSET ?4",
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let rows = stmt.query_map(params![query, repo_id, lim, off], |r| {
        Ok(SearchHit { id: r.get(0)?, slug: r.get(1)?, rank: r.get::<_, f64>(2).unwrap_or(0.0), title_snip: r.get::<_, String>(3).unwrap_or_default(), body_snip: r.get::<_, String>(4).unwrap_or_default() })
    }).map_err(|e| e.to_string())?;
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

#[derive(Serialize)]
pub struct GraphDoc { pub id: String, pub slug: String, pub title: String }

#[tauri::command]
pub async fn graph_backlinks(doc_id: String, db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<GraphDoc>, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare(
        "SELECT d.id, d.slug, d.title FROM link l JOIN doc d ON d.id = l.from_doc_id WHERE l.to_doc_id = ?1 ORDER BY d.updated_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![doc_id], |r| Ok(GraphDoc { id: r.get(0)?, slug: r.get(1)?, title: r.get(2)? })).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

#[tauri::command]
pub async fn graph_neighbors(doc_id: String, depth: Option<u8>, db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<GraphDoc>, String> {
    // 1-hop for now per plan; can expand depth later
    let conn = db.0.lock();
    let mut stmt = conn.prepare(
        "SELECT DISTINCT d.id, d.slug, d.title FROM (
            SELECT l2.from_doc_id AS neighbor_id FROM link l
            JOIN link l2 ON l.to_doc_id = l2.to_doc_id
            WHERE l.from_doc_id = ?1 AND l2.from_doc_id != ?1
            UNION
            SELECT to_doc_id FROM link WHERE from_doc_id = ?1 AND to_doc_id IS NOT NULL
        ) n JOIN doc d ON d.id = n.neighbor_id"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![doc_id], |r| Ok(GraphDoc { id: r.get(0)?, slug: r.get(1)?, title: r.get(2)? })).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

#[tauri::command]
pub async fn graph_related(doc_id: String, db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<GraphDoc>, String> {
    // Co-citation: docs that link to the same targets as doc_id
    let conn = db.0.lock();
    let mut stmt = conn.prepare(
        "SELECT d2.id, d2.slug, d2.title, COUNT(*) as score
         FROM link l1
         JOIN link l2 ON l1.to_doc_id = l2.to_doc_id
         JOIN doc d2 ON d2.id = l2.from_doc_id
         WHERE l1.from_doc_id = ?1 AND l2.from_doc_id != ?1 AND l2.from_doc_id IS NOT NULL
         GROUP BY d2.id
         ORDER BY score DESC, d2.updated_at DESC
         LIMIT 20"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![doc_id], |r| Ok(GraphDoc { id: r.get(0)?, slug: r.get(1)?, title: r.get(2)? })).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

#[tauri::command]
pub async fn graph_path(start_id: String, end_id: String, db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<String>, String> {
    let conn = db.0.lock();
    let sql = "WITH RECURSIVE
      path(n, path) AS (
        SELECT ?1, json_array(?1)
        UNION ALL
        SELECT l.to_doc_id, json_insert(path.path, '$[#]', l.to_doc_id)
        FROM link l JOIN path ON l.from_doc_id = path.n
        WHERE l.to_doc_id IS NOT NULL
          AND json_array_length(path.path) < 12
          AND NOT EXISTS (SELECT 1 FROM json_each(path.path) WHERE value = l.to_doc_id)
      )
    SELECT path FROM path WHERE n = ?2 LIMIT 1;";
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let path_json: Option<String> = stmt.query_row(params![start_id, end_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
    if let Some(p) = path_json {
        let v: serde_json::Value = serde_json::from_str(&p).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        if let Some(arr) = v.as_array() { for x in arr { if let Some(s) = x.as_str() { out.push(s.to_string()) } } }
        return Ok(out);
    }
    Ok(vec![])
}

// -------- AI Run ---------
#[derive(serde::Deserialize, Clone)]
pub struct AiRunRequest {
    pub provider: String,
    pub doc_id: String,
    pub anchor_id: Option<String>,
    pub line: Option<usize>,
    pub prompt: String,
}

#[tauri::command]
pub async fn ai_run(provider: String, doc_id: String, anchor_id: Option<String>, prompt: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let req = AiRunRequest { provider, doc_id, anchor_id, line: None, prompt };
    ai_run_core(&db, req)
}

pub fn ai_run_core(db: &std::sync::Arc<Db>, req: AiRunRequest) -> Result<serde_json::Value, String> {
    // Resolve provider: if empty or "default", use repo.settings.default_provider; else use provided
    let (body, provider_name): (String, String) = {
        let conn = db.0.lock();
        // fetch body and repo_id
        let (body, repo_id): (String, String) = conn.query_row(
            "SELECT df.body, d.repo_id FROM doc_fts df JOIN doc d ON d.rowid=df.rowid WHERE d.id=?1 OR d.slug=?1",
            rusqlite::params![req.doc_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).map_err(|e| e.to_string())?;
        let use_default = req.provider.is_empty() || req.provider == "default";
        let provider = if use_default {
            // repo default, else global app default, else 'local'
            let repo_default: Option<String> = conn.query_row(
                "SELECT json_extract(settings,'$.default_provider') FROM repo WHERE id=?1",
                rusqlite::params![repo_id],
                |r| r.get(0),
            ).ok();
            if let Some(p) = repo_default.filter(|s: &String| !s.is_empty()) { p } else {
                conn.query_row(
                    "SELECT value FROM app_setting WHERE key='default_provider'",
                    [],
                    |r| r.get::<_, String>(0),
                ).unwrap_or_else(|_| "local".into())
            }
        } else { req.provider.clone() };
        (body, provider)
    };

    // Determine target line
    let mut line = req.line.unwrap_or(1);
    if let Some(aid) = &req.anchor_id {
        if let Some(parsed) = parse_anchor_line(aid) { line = parsed; }
    }

    let context = extract_context(&body, line, 12);
    let redacted = redact(&context);

    // Provider gating and simulated response (echo)
    {
        let conn = db.0.lock();
        if let Ok((kind, enabled)) = conn.query_row(
            "SELECT kind, enabled FROM provider WHERE name=?1",
            rusqlite::params![&provider_name],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        ) {
            if enabled == 0 { return Err("provider_disabled".into()); }
            if kind == "remote" {
                drop(conn);
                if !crate::secrets::provider_key_exists(db, &provider_name)? {
                    return Err("no_key".into());
                }
            }
        }
    }
    let response_text = if provider_name == "openrouter" {
        format!("[openrouter]\nPrompt: {}\n---\n{}", req.prompt, redacted)
    } else {
        format!("[{}]\nPrompt: {}\n---\n{}", provider_name, req.prompt, redacted)
    };

    // Persist ai_trace
    let conn = db.0.lock();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let request_json = serde_json::json!({"prompt": req.prompt, "context": redacted});
    let response_json = serde_json::json!({"text": response_text});
    conn.execute(
        "INSERT INTO ai_trace(id,repo_id,doc_id,anchor_id,provider,request,response,input_tokens,output_tokens,cost_usd) VALUES(?, (SELECT repo_id FROM doc WHERE id=?2 OR slug=?2), ?2, ?, ?, ?, ?, 0, 0, 0.0)",
        rusqlite::params![trace_id, req.doc_id, req.anchor_id.unwrap_or_default(), req.provider, request_json.to_string(), response_json.to_string()],
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"trace_id": trace_id, "text": response_text}))
}

fn parse_anchor_line(anchor_id: &str) -> Option<usize> {
    // Expected formats: anc_<doc>_<line> or anc_<doc>_<line>_<ver>
    let parts: Vec<&str> = anchor_id.split('_').collect();
    if parts.len() >= 3 {
        parts[parts.len()-2].parse::<usize>().ok().or_else(|| parts.last()?.parse::<usize>().ok())
    } else { None }
}

// -------- Plugins (DB-backed) ---------
#[tauri::command]
pub async fn plugins_list(db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare("SELECT id,name,version,kind,enabled FROM plugin ORDER BY name ASC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(serde_json::json!({
        "id": r.get::<_, String>(0)?,
        "name": r.get::<_, String>(1)?,
        "version": r.get::<_, String>(2)?,
        "kind": r.get::<_, String>(3)?,
        "enabled": r.get::<_, i64>(4)? != 0
    }))).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

// -------- Provider Keys (stub: stored in provider.config) ---------
#[tauri::command]
pub async fn ai_provider_key_set(name: String, key: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let ok = secrets::provider_key_set(&db, &name, &key)?;
    Ok(serde_json::json!({"updated": ok}))
}

#[tauri::command]
pub async fn ai_provider_key_get(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let has = secrets::provider_key_exists(&db, &name)?;
    Ok(serde_json::json!({"has_key": has}))
}

#[tauri::command]
pub async fn ai_provider_test(name: String, prompt: Option<String>, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let res = ai::provider_test(&db, &name, &prompt.unwrap_or_else(|| "ping".into()))?;
    Ok(res)
}

#[tauri::command]
pub async fn plugins_enable(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("UPDATE plugin SET enabled=1 WHERE name=?1", params![name]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn plugins_disable(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("UPDATE plugin SET enabled=0 WHERE name=?1", params![name]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn plugins_info(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare("SELECT id,name,version,kind,manifest,permissions,enabled,installed_at FROM plugin WHERE name=?1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![name]).map_err(|e| e.to_string())?;
    if let Some(r) = rows.next().map_err(|e| e.to_string())? {
        return Ok(serde_json::json!({
            "id": r.get::<_, String>(0).unwrap_or_default(),
            "name": r.get::<_, String>(1).unwrap_or_default(),
            "version": r.get::<_, String>(2).unwrap_or_default(),
            "kind": r.get::<_, String>(3).unwrap_or_default(),
            "manifest": r.get::<_, String>(4).unwrap_or_default(),
            "permissions": r.get::<_, String>(5).unwrap_or_default(),
            "enabled": r.get::<_, i64>(6).unwrap_or(0) != 0,
            "installed_at": r.get::<_, String>(7).unwrap_or_default(),
        }))
    }
    Err("not_found".into())
}

#[tauri::command]
pub async fn plugins_remove(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("DELETE FROM plugin WHERE name=?1", params![name]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"removed": n>0}))
}

#[tauri::command]
pub async fn plugins_upsert(name: String, kind: Option<String>, version: Option<String>, permissions: Option<String>, enabled: Option<bool>, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let kind = kind.unwrap_or_else(|| "core".to_string());
    let version = version.unwrap_or_else(|| "dev".to_string());
    let perms = permissions.unwrap_or_else(|| "{}".to_string());
    let enabled = enabled.unwrap_or(true);
    let conn = db.0.lock();
    let n = conn.execute(
        "INSERT INTO plugin(id,name,version,kind,manifest,permissions,enabled) VALUES(?, ?, ?, ?, json('{}'), ?, ?) ON CONFLICT(name) DO UPDATE SET permissions=excluded.permissions, enabled=excluded.enabled, version=excluded.version, kind=excluded.kind",
        params![uuid::Uuid::new_v4().to_string(), name, version, kind, perms, if enabled {1} else {0}],
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"upserted": n>0}))
}

// -------- Core Plugin spawn/stop/call ---------
struct CoreProc { child: Child, stdin: Option<ChildStdin>, stdout: Option<BufReader<ChildStdout>> }

#[tauri::command]
pub async fn plugins_spawn_core(name: String, exec: String, args: Option<Vec<String>>) -> Result<serde_json::Value, String> {
    static REG: OnceLock<Mutex<HashMap<String, CoreProc>>> = OnceLock::new();
    let reg = REG.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = reg.lock().map_err(|_| "lock_poison")?;
    if map.contains_key(&name) {
        return Err("already_running".into());
    }
    let mut cmd = OsCommand::new(&exec);
    if let Some(a) = args.as_ref() { cmd.args(a); }
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    let pid = child.id();
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().map(BufReader::new);
    map.insert(name.clone(), CoreProc { child, stdin, stdout });
    Ok(serde_json::json!({"pid": pid}))
}

#[tauri::command]
pub async fn plugins_shutdown_core(name: String) -> Result<serde_json::Value, String> {
    static REG: OnceLock<Mutex<HashMap<String, CoreProc>>> = OnceLock::new();
    let reg = REG.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = reg.lock().map_err(|_| "lock_poison")?;
    if let Some(mut proc) = map.remove(&name) {
        let _ = proc.child.kill();
        let _ = proc.child.wait();
        return Ok(serde_json::json!({"stopped": true}));
    }
    Ok(serde_json::json!({"stopped": false}))
}

#[tauri::command]
pub async fn plugins_call_core(name: String, line: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    // Capability gate: plugin must be enabled and have permissions.core.call=true
    {
        let conn = db.0.lock();
        let allowed: i64 = conn
            .query_row(
                "SELECT CASE WHEN enabled=1 AND COALESCE(json_extract(permissions,'$.core.call'),0)=1 THEN 1 ELSE 0 END FROM plugin WHERE name=?1",
                params![name],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if allowed == 0 { return Err("forbidden".into()); }
    }
    // Validate JSON-RPC envelope and method-level permissions
    let parsed: serde_json::Value = serde_json::from_str(&line).map_err(|_| "invalid_request".to_string())?;
    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
    if method.is_empty() { return Err("invalid_request".into()); }
    let perm_key: Option<&str> = if method.starts_with("fs.write") {
        Some("$.fs.write")
    } else if method.starts_with("fs.") {
        Some("$.fs.read")
    } else if method.starts_with("net.request") {
        Some("$.net.request")
    } else if method.starts_with("db.write") {
        Some("$.db.write")
    } else if method.starts_with("db.") {
        Some("$.db.query")
    } else if method.starts_with("ai.invoke") {
        Some("$.ai.invoke")
    } else if method.starts_with("scanner.register") {
        Some("$.scanner.register")
    } else {
        None
    };
    if let Some(key) = perm_key {
        let conn = db.0.lock();
        let allowed: i64 = conn
            .query_row(
                "SELECT CASE WHEN enabled=1 AND COALESCE(json_extract(permissions,?1),0)=1 THEN 1 ELSE 0 END FROM plugin WHERE name=?2",
                params![key, name],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if allowed == 0 { return Err("forbidden".into()); }
    }
    // net.request domain allowlist: permissions.net.domains contains allowed hosts
    if method.starts_with("net.request") {
        let params_v = parsed.get("params").cloned().unwrap_or(serde_json::json!({}));
        let url_s = params_v.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if !url_s.is_empty() {
            // naive host extraction
            let host = if let Some(rest) = url_s.split("//").nth(1) {
                rest.split('/').next().unwrap_or("").split(':').next().unwrap_or("")
            } else { url_s };
            let conn = db.0.lock();
            let perms_json: Option<String> = conn.query_row("SELECT permissions FROM plugin WHERE name=?1", params![name], |r| r.get(0)).ok();
            let mut allowed = false;
            if let Some(pj) = perms_json {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&pj) {
                    if let Some(arr) = val.get("net").and_then(|n| n.get("domains")).and_then(|d| d.as_array()) {
                        for d in arr {
                            if let Some(dom) = d.as_str() {
                                if host.eq_ignore_ascii_case(dom) || (dom.starts_with('.') && host.ends_with(dom)) {
                                    allowed = true; break;
                                }
                            }
                        }
                    }
                }
            }
            if !allowed { return Err("forbidden_net_domain".into()); }
        }
    }
    // FS roots allowlist: when calling fs.* methods, enforce that params.path is under one of permissions.fs.roots
    if method.starts_with("fs.") {
        let params_v = parsed.get("params").cloned().unwrap_or(serde_json::json!({}));
        let req_path = params_v.get("path").and_then(|v| v.as_str()).unwrap_or("");
        if !req_path.is_empty() {
            let conn = db.0.lock();
            let perms_json: Option<String> = conn.query_row("SELECT permissions FROM plugin WHERE name=?1", params![name], |r| r.get(0)).ok();
            let mut allowed = false;
            if let Some(pj) = perms_json {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&pj) {
                    if let Some(arr) = val.get("fs").and_then(|fs| fs.get("roots")).and_then(|r| r.as_array()) {
                        let target_canon: PathBuf = Path::new(req_path).canonicalize().unwrap_or_else(|_| PathBuf::from(req_path));
                        for root in arr {
                            if let Some(root_s) = root.as_str() {
                                let root_canon: PathBuf = Path::new(root_s).canonicalize().unwrap_or_else(|_| PathBuf::from(root_s));
                                if target_canon.starts_with(&root_canon) { allowed = true; break; }
                            }
                        }
                    }
                }
            }
            if !allowed { return Err("forbidden_fs_root".into()); }
        }
    }
    static REG: OnceLock<Mutex<HashMap<String, CoreProc>>> = OnceLock::new();
    let reg = REG.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = reg.lock().map_err(|_| "lock_poison")?;
    if let Some(proc) = map.get_mut(&name) {
        if let Some(stdin) = proc.stdin.as_mut() {
            stdin.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
            stdin.write_all(b"\n").map_err(|e| e.to_string())?;
            stdin.flush().ok();
        } else { return Err("stdin_closed".into()); }
        if let Some(stdout) = proc.stdout.as_mut() {
            let mut buf = String::new();
            stdout.read_line(&mut buf).map_err(|e| e.to_string())?;
            let trimmed = buf.trim();
            if trimmed.is_empty() { return Ok(serde_json::json!({"ok": true})); }
            let val: serde_json::Value = serde_json::from_str(trimmed).unwrap_or(serde_json::json!({"line": trimmed}));
            return Ok(val);
        } else { return Err("stdout_closed".into()); }
    }
    Err("not_found".into())
}

// -------- AI Providers ---------
#[derive(Serialize)]
pub struct ProviderRow { pub name: String, pub kind: String, pub enabled: bool }

#[tauri::command]
pub async fn ai_providers_list(db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<ProviderRow>, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare("SELECT name, kind, enabled FROM provider ORDER BY name ASC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(ProviderRow { name: r.get(0)?, kind: r.get(1)?, enabled: r.get::<_, i64>(2)? != 0 })).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

#[tauri::command]
pub async fn ai_providers_enable(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("UPDATE provider SET enabled=1, updated_at=datetime('now') WHERE name=?1", params![name]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn ai_providers_disable(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("UPDATE provider SET enabled=0, updated_at=datetime('now') WHERE name=?1", params![name]).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

fn extract_context(body: &str, line: usize, n: usize) -> String {
    let lines: Vec<&str> = body.lines().collect();
    if lines.is_empty() { return String::new(); }
    let idx = if line == 0 { 0 } else { line - 1 };
    let start = idx.saturating_sub(n);
    let end = (idx + n + 1).min(lines.len());
    lines[start..end].join("\n")
}

fn redact(s: &str) -> String {
    let mut out = s.to_string();
    // simple patterns
    out = out.replace("AKIA", "****");
    out = out.replace("api_key", "****");
    out
}

#[tauri::command]
pub async fn anchors_upsert(doc_id: String, anchor_id: String, line: i64, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let id = uuid::Uuid::new_v4().to_string();
    let meta = serde_json::json!({"doc_id": doc_id, "line": line});
    conn.execute(
        "INSERT INTO provenance(id,entity_type,entity_id,source,meta) VALUES(?, 'anchor', ?, 'ui', ?)",
        rusqlite::params![id, anchor_id, meta.to_string()],
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"ok": true}))
}

#[derive(Serialize)]
pub struct AnchorItem { pub id: String, pub line: i64, pub created_at: String }

#[tauri::command]
pub async fn anchors_list(doc_id: String, db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<AnchorItem>, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare(
        "SELECT entity_id, COALESCE(json_extract(meta,'$.line'), 0), created_at \
         FROM provenance WHERE entity_type='anchor' AND json_extract(meta,'$.doc_id')=?1 \
         ORDER BY created_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![doc_id], |r| Ok(AnchorItem { id: r.get(0)?, line: r.get(1)?, created_at: r.get(2)? }))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?) }
    Ok(out)
}

#[tauri::command]
pub async fn anchors_delete(anchor_id: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn.execute("DELETE FROM provenance WHERE entity_type='anchor' AND entity_id=?1", params![anchor_id])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"deleted": n>0}))
}
