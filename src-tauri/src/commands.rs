use crate::{db::Db, scan};
use tauri::Emitter;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

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
    tx.execute("INSERT INTO doc_version(id,doc_id,blob_id,hash) VALUES(?,?,?,?)", params![version_id, doc_id, blob_id, version_id]).map_err(|e| e.to_string())?;
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
    let version_id = Uuid::new_v4().to_string();
    let blob_id = Uuid::new_v4().to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)", params![blob_id, payload.body.as_bytes(), payload.body.len() as i64]).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO doc_version(id,doc_id,blob_id,hash,message) VALUES(?,?,?,?,?)", params![version_id, payload.doc_id, blob_id, version_id, payload.message.unwrap_or_default()]).map_err(|e| e.to_string())?;
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
    // Fetch body
    let conn = db.0.lock();
    let body: String = conn.query_row(
        "SELECT body FROM doc_fts WHERE rowid=(SELECT rowid FROM doc WHERE id=?1 OR slug=?1)",
        rusqlite::params![req.doc_id],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    drop(conn);

    // Determine target line
    let mut line = req.line.unwrap_or(1);
    if let Some(aid) = &req.anchor_id {
        if let Some(parsed) = parse_anchor_line(aid) { line = parsed; }
    }

    let context = extract_context(&body, line, 12);
    let redacted = redact(&context);

    // Simulated provider response (echo)
    let response_text = format!("[{}]\nPrompt: {}\n---\n{}", req.provider, req.prompt, redacted);

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
