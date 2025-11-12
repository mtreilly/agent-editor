use crate::db::Db;
use tauri::Emitter;
use ignore::{overrides::OverrideBuilder, WalkBuilder};
use rusqlite::{params, OptionalExtension};
use std::{fs, path::{Path, PathBuf}};
use uuid::Uuid;

#[derive(Default, Debug, Clone)]
pub struct ScanStats { pub files_scanned: i64, pub docs_added: i64, pub errors: i64 }

pub fn scan_once(db: &Db, repo_path: &str, include: &[String], exclude: &[String]) -> Result<ScanStats, String> {
    let mut stats = ScanStats::default();
    let repo_path = PathBuf::from(repo_path);
    if !repo_path.exists() { return Err("repo path not found".into()); }

    let mut ov = OverrideBuilder::new(&repo_path);
    for g in include { ov.add(g).map_err(|e| e.to_string())?; }
    for g in exclude { ov.add(&format!("!{}", g)).map_err(|e| e.to_string())?; }
    let overrides = ov.build().map_err(|e| e.to_string())?;

    let mut walker = WalkBuilder::new(&repo_path);
    walker.hidden(true).git_ignore(true).git_global(true).git_exclude(true).overrides(overrides);
    let walker = walker.build();

    let debug = std::env::var("AE_DEBUG_SCAN").ok().map(|v| v=="1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
    for res in walker {
        match res {
            Ok(entry) => {
                let path = entry.path();
                if path.is_dir() { continue; }
                if path.extension().and_then(|s| s.to_str()).unwrap_or("") != "md" { continue; }
                stats.files_scanned += 1;
                match upsert_doc(&db, &repo_path, path) {
                    Ok(added) => if added { stats.docs_added += 1; },
                    Err(e) => { if debug { eprintln!("[scan] upsert error for {}: {}", path.display(), e); } stats.errors += 1; },
                }
            }
            Err(e) => { stats.errors += 1; if debug { eprintln!("[scan] walk error: {}", e); } }
        }
    }
    Ok(stats)
}

/// Scan a single file path (absolute) under a given repo_root (absolute).
pub fn scan_one_file(db: &Db, repo_root: &str, file_path: &str) -> Result<bool, String> {
    let root = PathBuf::from(repo_root);
    let fp = PathBuf::from(file_path);
    if !fp.exists() { return Err("file not found".into()); }
    if fp.extension().and_then(|s| s.to_str()).unwrap_or("") != "md" { return Ok(false); }
    upsert_doc(db, &root, &fp)
}

fn upsert_doc(db: &Db, repo_root: &Path, file_path: &Path) -> Result<bool, String> {
    let content = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
    let slug = make_slug(repo_root, file_path);
    let size = content.len() as i64;
    let lines = content.lines().count() as i64;
    let mut conn = db.0.lock();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Ensure repo exists and get id by path
    let repo_path_str = repo_root.to_string_lossy().to_string();
    let repo_id: Option<String> = tx.query_row("SELECT id FROM repo WHERE path=?1", params![repo_path_str], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
    let repo_id = repo_id.unwrap_or_else(|| {
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT OR IGNORE INTO repo(id,name,path) VALUES(?,?,?)", params![id.clone(), repo_root.file_name().and_then(|s| s.to_str()).unwrap_or("") , repo_path_str]).ok();
        id
    });

    // Ensure folder
    let rel = file_path.strip_prefix(repo_root).unwrap_or(file_path);
    let folder_path = rel.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "".into());
    let folder_id: String = {
        // find existing folder
        let fid: Option<String> = tx.query_row("SELECT id FROM folder WHERE repo_id=?1 AND path=?2", params![repo_id, folder_path], |r| r.get(0)).optional().unwrap_or(None);
        if let Some(fid) = fid { fid } else {
            let fid = Uuid::new_v4().to_string();
            let fslug = folder_slug(&folder_path);
            tx.execute("INSERT INTO folder(id,repo_id,path,slug) VALUES(?,?,?,?)", params![fid, repo_id, folder_path, fslug]).map_err(|e| e.to_string())?;
            fid
        }
    };

    // Upsert doc by (repo_id, slug)
    let doc_id_opt: Option<String> = tx.query_row("SELECT id FROM doc WHERE repo_id=?1 AND slug=?2", params![repo_id, slug], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
    let (doc_id, is_new_doc) = if let Some(id) = doc_id_opt { (id, false) } else {
        let id = Uuid::new_v4().to_string();
        let title = rel.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        tx.execute("INSERT INTO doc(id,repo_id,folder_id,slug,title,size_bytes,line_count) VALUES(?,?,?,?,?,?,?)",
            params![id, repo_id, folder_id, slug, title, size, lines]).ok();
        (id, true)
    };

    // Dedupe against current version hash
    let version_hash = format!("{}:{}", doc_id, content_hash);
    let mut changed = true;
    if !is_new_doc {
        if let Ok(prev_hash) = tx.query_row(
            "SELECT v.hash FROM doc d JOIN doc_version v ON v.id = d.current_version_id WHERE d.id=?1",
            params![&doc_id],
            |r| r.get::<_, String>(0),
        ) {
            if prev_hash == version_hash { changed = false; }
        }
    }

    if changed {
        // Append version
        let blob_id = Uuid::new_v4().to_string();
        let version_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO doc_blob(id,content,size_bytes) VALUES(?,?,?)", params![blob_id, content.as_bytes(), size]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO doc_version(id,doc_id,blob_id,hash) VALUES(?,?,?,?)", params![version_id, doc_id, blob_id, version_hash]).map_err(|e| e.to_string())?;
        tx.execute("UPDATE doc SET current_version_id=?1, size_bytes=?2, line_count=?3, updated_at=datetime('now') WHERE id=?4", params![version_id, size, lines, doc_id]).map_err(|e| e.to_string())?;
        // Update FTS
    tx.execute("INSERT INTO doc_fts(doc_fts,rowid) VALUES('delete',(SELECT rowid FROM doc WHERE id=?1))", params![doc_id]).ok();
    tx.execute("INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid,d.title,?1,d.slug,d.repo_id FROM doc d WHERE d.id=?2", params![content, doc_id]).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    // release connection lock before link update to avoid deadlock
    drop(conn);
    // update links only if new or changed
    if changed || is_new_doc {
        crate::graph::update_links_for_doc(&db.0.lock(), &doc_id, &content)?;
    }
    Ok(is_new_doc || changed)
}

fn make_slug(repo_root: &Path, file_path: &Path) -> String {
    let rel = file_path.strip_prefix(repo_root).unwrap_or(file_path);
    let mut s = rel.with_extension("").to_string_lossy().to_string();
    s = s.replace(std::path::MAIN_SEPARATOR, "__");
    s = s.replace(' ', "-");
    s
}

fn folder_slug(path: &str) -> String {
    if path.is_empty() { return String::from("") }
    let p = Path::new(path);
    let last = p.file_name().and_then(|s| s.to_str()).unwrap_or(path);
    last.replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_make_slug() {
        let root = PathBuf::from("/repo");
        let fp = PathBuf::from("/repo/notes/Deep Topic.md");
        let slug = make_slug(&root, &fp);
        // relative path without extension, separators replaced with '__', spaces to '-'
        assert_eq!(slug, "notes__Deep-Topic");
    }
}

// Watch filesystem for changes under repo_path and rescan modified markdown files.
pub fn watch_repo(db: std::sync::Arc<Db>, repo_path: String, include: Vec<String>, exclude: Vec<String>, debounce_ms: u64, app: tauri::AppHandle) -> Result<(), String> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::channel;
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).map_err(|e| e.to_string())?;
    watcher.watch(Path::new(&repo_path), RecursiveMode::Recursive).map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        let mut last = std::collections::HashMap::<PathBuf, std::time::Instant>::new();
        loop {
            if let Ok(event) = rx.recv() {
                let evt = match event { Ok(e) => e, Err(_) => continue };
                let paths = evt.paths;
                for p in paths {
                    if p.is_dir() { continue; }
                    if p.extension().and_then(|s| s.to_str()).unwrap_or("") != "md" { continue; }
                    let now = std::time::Instant::now();
                    let prev = last.get(&p).cloned();
                    if let Some(t) = prev { if now.duration_since(t).as_millis() < debounce_ms as u128 { continue; } }
                    last.insert(p.clone(), now);
                    // Optional include/exclude matching via ignore::Override
                    let mut ov = ignore::overrides::OverrideBuilder::new(&repo_path);
                    for g in &include { let _ = ov.add(g); }
                    for g in &exclude { let _ = ov.add(&format!("!{}", g)); }
                    if let Ok(ovm) = ov.build() {
                        if !ovm.matched(&p, false).is_whitelist() { continue; }
                    }
                    // Rescan one file
                    let _ = upsert_doc(&db, Path::new(&repo_path), &p);
                    let _ = app.emit("progress.scan", serde_json::json!({
                        "event": match evt.kind { EventKind::Create(_) => "create", EventKind::Modify(_) => "modify", EventKind::Remove(_) => "remove", _ => "other" },
                        "path": p.to_string_lossy(),
                    }));
                }
            }
        }
    });
    Ok(())
}
