//! Repository scanning commands

use crate::{db::Db, scan};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ScanFilters {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ScanJobReport {
    pub job_id: String,
    pub files_scanned: i64,
    pub docs_added: i64,
    pub errors: i64,
}

#[tauri::command]
pub async fn scan_repo(
    repo_path: String,
    filters: Option<ScanFilters>,
    watch: Option<bool>,
    debounce: Option<u64>,
    db: State<'_, std::sync::Arc<Db>>,
    app: tauri::AppHandle,
) -> Result<ScanJobReport, String> {
    let job_id = Uuid::new_v4().to_string();
    let repo_id = {
        let conn = db.0.lock();
        let mut stmt = conn
            .prepare("SELECT id FROM repo WHERE path=?1 OR name=?1")
            .map_err(|e| e.to_string())?;
        let id: Option<String> = stmt
            .query_row(params![repo_path], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        id.unwrap_or_else(|| {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO repo(id,name,path) VALUES(?,?,?)",
                params![id, &repo_path, &repo_path],
            )
            .ok();
            id
        })
    };
    let conn = db.0.lock();
    conn.execute(
        "INSERT INTO scan_job(id,repo_id,status,stats) VALUES(?,?,'running',json('{}'))",
        params![job_id, repo_id],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);
    let include = filters
        .as_ref()
        .and_then(|f| f.include.clone())
        .unwrap_or_default();
    let exclude = filters
        .as_ref()
        .and_then(|f| f.exclude.clone())
        .unwrap_or_default();
    let stats = scan::scan_once(&db, &repo_path, &include, &exclude)?;
    let conn2 = db.0.lock();
    conn2
        .execute(
            "UPDATE scan_job SET status='success', stats=?2, finished_at=datetime('now') WHERE id=?1",
            params![
                job_id,
                serde_json::to_string(&serde_json::json!({
                    "files_scanned": stats.files_scanned,
                    "docs_added": stats.docs_added,
                    "errors": stats.errors
                }))
                .unwrap()
            ],
        )
        .map_err(|e| e.to_string())?;
    if watch.unwrap_or(false) {
        let _ = app.emit(
            "progress.scan",
            serde_json::json!({"event": "watch-start", "path": repo_path}),
        );
        let _ = scan::watch_repo(
            db.inner().clone(),
            repo_path.clone(),
            include,
            exclude,
            debounce.unwrap_or(200),
            app,
        );
    }
    Ok(ScanJobReport {
        job_id,
        files_scanned: stats.files_scanned,
        docs_added: stats.docs_added,
        errors: stats.errors,
    })
}
