use crate::{db::Db, scan};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use crate::secrets;
use crate::ai;
use tauri::Emitter;
use rusqlite::{params, OptionalExtension, Connection};
use rusqlite::backup::Backup;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use std::collections::HashMap;
use std::process::{Child, ChildStdin, ChildStdout, Command as OsCommand, Stdio};
use std::sync::{Mutex, OnceLock};
use std::path::{Path, PathBuf};
use std::io::{Write, BufRead, BufReader, Read};
use std::fs::{File, OpenOptions};
use tar::Archive;

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

#[cfg(test)]
mod tests_perm {
    use super::*;
    use crate::db::open_db;
    use std::sync::Arc;

    fn test_db() -> Arc<Db> {
        let p = std::env::temp_dir().join(format!("ae-perm-test-{}.db", uuid::Uuid::new_v4()));
        Arc::new(open_db(&p).expect("open db"))
    }

    fn insert_plugin(db: &Arc<Db>, name: &str, enabled: i64, permissions: &str) {
        let conn = db.0.lock();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO plugin(id,name,version,kind,manifest,permissions,enabled) VALUES(?1,?2,'1.0.0','core',json('{}'),?3,?4) \
             ON CONFLICT(name) DO UPDATE SET permissions=excluded.permissions, enabled=excluded.enabled",
            params![id, name, permissions, enabled],
        ).unwrap();
    }

    #[test]
    fn core_call_forbidden_when_disabled() {
        let db = test_db();
        insert_plugin(&db, "p1", 0, r#"{"core":{"call":1}}"#);
        let line = r#"{"jsonrpc":"2.0","id":"1","method":"fs.readFile","params":{"path":"/tmp"}}"#;
        let err = plugins_call_core_check(&db, "p1", line).unwrap_err();
        assert_eq!(err, "forbidden");
    }

    #[test]
    fn core_call_forbidden_without_permission() {
        let db = test_db();
        insert_plugin(&db, "p2", 1, r#"{}"#);
        let line = r#"{"jsonrpc":"2.0","id":"1","method":"fs.readFile","params":{"path":"/tmp"}}"#;
        let err = plugins_call_core_check(&db, "p2", line).unwrap_err();
        assert_eq!(err, "forbidden");
    }

    #[test]
    fn net_request_domain_allowlist() {
        let db = test_db();
        insert_plugin(&db, "p3", 1, r#"{"core":{"call":1},"net":{"request":1,"domains":["api.example.com"]}}"#);
        let ok_line = r#"{"jsonrpc":"2.0","id":"1","method":"net.request","params":{"url":"https://api.example.com/v1"}}"#;
        let bad_line = r#"{"jsonrpc":"2.0","id":"1","method":"net.request","params":{"url":"https://other.com/"}}"#;
        assert!(plugins_call_core_check(&db, "p3", ok_line).is_ok());
        let err = plugins_call_core_check(&db, "p3", bad_line).unwrap_err();
        assert_eq!(err, "forbidden_net_domain");
    }

    #[test]
    fn fs_roots_allowlist() {
        let db = test_db();
        let root = std::env::temp_dir().join(format!("ae-perm-root-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let inside = root.join("file.txt");
        std::fs::write(&inside, b"ok").unwrap();
        let perms = format!("{{\"core\":{{\"call\":1}},\"fs\":{{\"read\":1,\"roots\":[\"{}\"]}}}}", root.display());
        insert_plugin(&db, "p4", 1, &perms);
        let ok_line = format!("{{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"method\":\"fs.readFile\",\"params\":{{\"path\":\"{}\"}}}}", inside.display());
        assert!(plugins_call_core_check(&db, "p4", &ok_line).is_ok());
        let bad_line = "{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"method\":\"fs.readFile\",\"params\":{\"path\":\"/etc/hosts\"}}";
        let err = plugins_call_core_check(&db, "p4", bad_line).unwrap_err();
        assert_eq!(err, "forbidden_fs_root");
    }

    #[test]
    fn redact_common_secrets() {
        // AWS AKIA and bearer tokens should be masked
        let input = "AWS key AKIAABCDEFGHIJKLMNOP token: Bearer abcdefghijklmnopqrstuvwxyz0123456789";
        let out = super::redact(input);
        assert!(!out.contains("AKIAABCDEFGHIJKLMNOP"));
        assert!(out.to_lowercase().contains("bearer ****"));

        // Query params
        let input2 = "https://example.com/?token=SeCrEtToKeN123456&x=1";
        let out2 = super::redact(input2);
        assert!(out2.contains("token=****"));
    }

    #[test]
    fn invalid_envelope_missing_method() {
        let db = test_db();
        insert_plugin(&db, "p7", 1, r#"{"core":{"call":1}}"#);
        // Missing method field
        let line = r#"{"jsonrpc":"2.0","id":"1","params":{}}"#;
        let err = plugins_call_core_check(&db, "p7", line).unwrap_err();
        assert_eq!(err, "invalid_request");
    }

    #[test]
    fn invalid_envelope_bad_json() {
        let db = test_db();
        insert_plugin(&db, "p8", 1, r#"{"core":{"call":1}}"#);
        // Not JSON
        let line = "this is not json";
        let err = plugins_call_core_check(&db, "p8", line).unwrap_err();
        assert_eq!(err, "invalid_request");
    }

    #[test]
    fn db_permissions_query_and_write() {
        let db = test_db();
        // Only query allowed
        insert_plugin(&db, "p5", 1, r#"{"core":{"call":1},"db":{"query":1}}"#);
        let q_line = r#"{"jsonrpc":"2.0","id":"1","method":"db.query","params":{"sql":"SELECT 1"}}"#;
        assert!(plugins_call_core_check(&db, "p5", q_line).is_ok());
        let w_line = r#"{"jsonrpc":"2.0","id":"1","method":"db.writeInsert","params":{"sql":"INSERT"}}"#;
        let err = plugins_call_core_check(&db, "p5", w_line).unwrap_err();
        assert_eq!(err, "forbidden");

        // Write allowed
        insert_plugin(&db, "p6", 1, r#"{"core":{"call":1},"db":{"write":1}}"#);
        assert!(plugins_call_core_check(&db, "p6", w_line).is_ok());
    }
}

#[cfg(test)]
mod tests_plugins {
    use super::*;
    use std::io::Write as _;

    fn has_node() -> bool {
        std::process::Command::new("node").arg("--version").output().is_ok()
    }

    #[test]
    fn core_call_watchdog_timeout() {
        if !has_node() { return; }
        // Spawn slow-core with 200ms response delay; set timeout to 50ms to trigger timeout
        std::env::set_var("SLOW_DELAY_MS", "200");
        let args = vec!["plugins/slow-core/slow.js".to_string()];
        let (mut child, mut stdin_opt, stdout_opt) = spawn_core_child("slow", "node", &args).expect("spawn slow-core");
        let mut stdin = stdin_opt.take().expect("has stdin");
        // Send a valid JSON-RPC line
        let line = r#"{"jsonrpc":"2.0","id":"1","method":"fs.read","params":{"path":"README.md"}}"#;
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();

        // Read one line with timeout (50ms)
        let stdout = stdout_opt.expect("has stdout");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stdout);
            let mut buf = String::new();
            let res = reader.read_line(&mut buf).map_err(|e| e.to_string());
            let _ = tx.send((res, buf));
        });
        let got = rx.recv_timeout(std::time::Duration::from_millis(50));
        // Ensure timeout (no line received in 50ms)
        assert!(got.is_err(), "expected timeout before slow-core responds");

        // Cleanup
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests_export {
    use super::*;
    use crate::db::open_db;
    use std::sync::Arc;

    fn test_db() -> Arc<Db> {
        let p = std::env::temp_dir().join(format!("ae-export-test-{}.db", uuid::Uuid::new_v4()));
        Arc::new(open_db(&p).expect("open db"))
    }

    fn ensure_repo(conn: &Connection, repo_id: &str) {
        let path = format!("/{}", repo_id);
        conn.execute(
            "INSERT OR IGNORE INTO repo(id,name,path,settings) VALUES(?,?,?,json('{}'))",
            params![repo_id, repo_id, path],
        ).unwrap();
        let folder_id = format!("folder-{}", repo_id);
        conn.execute(
            "INSERT OR IGNORE INTO folder(id,repo_id,parent_id,path,slug) VALUES(?,?,?,?,?)",
            params![folder_id, repo_id, Option::<String>::None, path.clone(), "root"],
        ).unwrap();
    }

    fn insert_doc(conn: &Connection, repo_id: &str, slug: &str, title: &str, body: &str, is_deleted: bool) {
        ensure_repo(conn, repo_id);
        let doc_id = uuid::Uuid::new_v4().to_string();
        let folder_id = format!("folder-{}", repo_id);
        conn.execute(
            "INSERT INTO doc(id,repo_id,folder_id,slug,title,is_deleted,current_version_id,size_bytes,line_count) VALUES(?,?,?,?,?,?,?,?,?)",
            params![
                doc_id,
                repo_id,
                folder_id,
                slug,
                title,
                if is_deleted { 1 } else { 0 },
                Option::<String>::None,
                body.len() as i64,
                body.lines().count() as i64,
            ],
        ).unwrap();
        conn.execute(
            "INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT rowid, ?1, ?2, ?3, ?4 FROM doc WHERE id=?5",
            params![title, body, slug, repo_id, doc_id],
        ).unwrap();
    }

    #[test]
    fn export_docs_filters_repo_and_deleted() {
        let db = test_db();
        {
            let conn = db.0.lock();
            insert_doc(&conn, "repo_a", "one", "Doc One", "Body", false);
            insert_doc(&conn, "repo_a", "two", "Doc Two", "Body", true);
            insert_doc(&conn, "repo_b", "three", "Doc Three", "Body", false);
        }
        let conn = db.0.lock();
        let repo_only = fetch_doc_exports(&conn, Some("repo_a"), false).unwrap();
        assert_eq!(repo_only.len(), 1);
        assert_eq!(repo_only[0].slug, "one");
        let with_deleted = fetch_doc_exports(&conn, Some("repo_a"), true).unwrap();
        assert_eq!(with_deleted.len(), 2);
        assert!(with_deleted.iter().any(|r| r.slug == "two" && r.is_deleted));
        let all = fetch_doc_exports(&conn, None, false).unwrap();
        assert_eq!(all.len(), 2);
    }
}

#[cfg(test)]
mod tests_import {
    use super::*;
    use crate::db::open_db;
    use std::fs::File;
    use std::sync::Arc;
    use tar::Builder;

    fn test_db() -> Arc<Db> {
        let p = std::env::temp_dir().join(format!("ae-import-test-{}.db", uuid::Uuid::new_v4()));
        Arc::new(open_db(&p).expect("open db"))
    }

    fn ensure_repo(conn: &Connection, repo_id: &str) -> String {
        let path = format!("/{}", repo_id);
        conn.execute(
            "INSERT OR IGNORE INTO repo(id,name,path,settings) VALUES(?,?,?,json('{}'))",
            params![repo_id, repo_id, path],
        ).unwrap();
        let folder_id = format!("folder-{}", repo_id);
        conn.execute(
            "INSERT OR IGNORE INTO folder(id,repo_id,parent_id,path,slug) VALUES(?,?,?,?,?)",
            params![folder_id, repo_id, Option::<String>::None, path, "root"],
        ).unwrap();
        folder_id
    }

    fn insert_doc(conn: &Connection, repo_id: &str, slug: &str, title: &str, body: &str) -> String {
        let folder_id = ensure_repo(conn, repo_id);
        let doc_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO doc(id,repo_id,folder_id,slug,title,is_deleted,current_version_id,size_bytes,line_count,created_at,updated_at) VALUES(?,?,?,?,?,?,NULL,?,?,datetime('now'),datetime('now'))",
            params![doc_id, repo_id, folder_id, slug, title, 0, body.len() as i64, body.lines().count() as i64],
        ).unwrap();
        conn.execute(
            "INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid, ?1, ?2, ?3, ?4 FROM doc d WHERE d.id=?5",
            params![title, body, slug, repo_id, doc_id],
        ).unwrap();
        doc_id
    }

    #[test]
    fn import_docs_round_trip_json() {
        let db = test_db();
        let export_path = std::env::temp_dir().join(format!("ae-import-roundtrip-{}.json", uuid::Uuid::new_v4()));
        let export_path_str = export_path.to_string_lossy().to_string();
        {
            let conn = db.0.lock();
            insert_doc(&conn, "repo_src", "welcome", "Welcome", "# Welcome\n\nBody", false);
        }
        {
            let conn = db.0.lock();
            let docs = fetch_doc_exports(&conn, Some("repo_src"), false).unwrap();
            let file = File::create(&export_path).unwrap();
            serde_json::to_writer(file, &docs).unwrap();
        }
        let docs = read_docs_from_path(export_path.as_path()).unwrap();
        {
            let conn = db.0.lock();
            let res = import_docs_apply(&conn, docs, None, Some("Imported Repo".into()), false, "keep", &export_path_str, None).unwrap();
            assert_eq!(res["inserted"].as_u64().unwrap(), 1);
            assert_eq!(res["status"].as_str(), Some("imported"));
        }
        {
            let conn = db.0.lock();
            let repo_id: String = conn
                .query_row("SELECT id FROM repo WHERE name='Imported Repo'", [], |r| r.get(0))
                .unwrap();
            let (doc_id, size_bytes, line_count): (String, i64, i64) = conn
                .query_row(
                    "SELECT id,size_bytes,line_count FROM doc WHERE repo_id=?1",
                    params![repo_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();
            assert!(size_bytes > 0);
            assert!(line_count > 0);
            let fts_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM doc_fts WHERE rowid=(SELECT rowid FROM doc WHERE id=?1)",
                    params![doc_id.clone()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(fts_count, 1);
            let version_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM doc_version WHERE doc_id=?1", params![doc_id.clone()], |r| r.get(0))
                .unwrap_or(0);
            assert_eq!(version_count, 1);
            let provenance_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM provenance WHERE entity_id=?1 AND source='import'",
                    params![doc_id.clone()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(provenance_count, 1);
        }
    }

    #[test]
    fn read_docs_from_tar_hydrates_missing_body() {
        let tar_path = std::env::temp_dir().join(format!("ae-import-docs-body-{}.tar", uuid::Uuid::new_v4()));
        let doc_id = uuid::Uuid::new_v4().to_string();
        let slug = "Guides/Getting Started!!!";
        let body = "# Imported body\n\nDetails";
        {
            let file = File::create(&tar_path).unwrap();
            let mut builder = Builder::new(file);

            let docs_payload = serde_json::json!([{
                "id": doc_id,
                "repo_id": "repo_tar",
                "slug": slug,
                "title": "Guide",
                "body": serde_json::Value::Null,
                "is_deleted": false
            }]);
            append_tar_bytes(&mut builder, "docs.json", serde_json::to_vec(&docs_payload).unwrap().as_slice());
            append_tar_bytes(
                &mut builder,
                "meta.json",
                serde_json::to_vec(&serde_json::json!({"doc_count": 1, "format": "json"})).unwrap().as_slice(),
            );
            let filename = format!("docs/{}-{}.md", sanitize_slug_for_filename(slug), doc_id);
            append_tar_bytes(&mut builder, &filename, body.as_bytes());
            builder.finish().unwrap();
        }

        let docs = read_docs_from_tar(&tar_path).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].body.as_deref(), Some(body));
    }

    fn append_tar_bytes(builder: &mut Builder<File>, name: &str, data: &[u8]) {
        let mut header = tar::Header::new_gnu();
        header.set_path(name).unwrap();
        header.set_size(data.len() as u64);
        header.set_mode(0o600);
        header.set_mtime(0);
        header.set_cksum();
        builder.append(&header, data).unwrap();
    }

    fn import_row(body: &str, message: &str) -> DocImportRow {
        DocImportRow {
            id: None,
            repo_id: None,
            slug: "guide".into(),
            title: Some("Guide".into()),
            body: Some(body.to_string()),
            is_deleted: Some(false),
            updated_at: None,
            versions: Some(vec![DocVersionImport {
                id: None,
                hash: None,
                created_at: None,
                message: Some(message.to_string()),
            }]),
        }
    }

    #[test]
    fn import_docs_respects_merge_strategy() {
        let db = test_db();
        {
            let conn = db.0.lock();
            insert_doc(&conn, "repo_merge", "guide", "Guide", "old body", false);
        }
        {
            let conn = db.0.lock();
            let summary = import_docs_apply(
                &conn,
                vec![import_row("new body", "keep")],
                Some("repo_merge".into()),
                None,
                false,
                "keep",
                "import.json",
                None,
            )
            .unwrap();
            assert_eq!(summary["skipped"].as_u64().unwrap(), 1);
            let body: String = conn
                .query_row(
                    "SELECT f.body FROM doc_fts f JOIN doc d ON d.rowid=f.rowid WHERE d.repo_id=?1 AND d.slug='guide'",
                    params!["repo_merge"],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(body.contains("old body"));
        }
        {
            let conn = db.0.lock();
            let summary = import_docs_apply(
                &conn,
                vec![import_row("## updated guide", "overwrite")],
                Some("repo_merge".into()),
                None,
                false,
                "overwrite",
                "import.json",
                None,
            )
            .unwrap();
            assert_eq!(summary["updated"].as_u64().unwrap(), 1);
            let body: String = conn
                .query_row(
                    "SELECT f.body FROM doc_fts f JOIN doc d ON d.rowid=f.rowid WHERE d.repo_id=?1 AND d.slug='guide'",
                    params!["repo_merge"],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(body.contains("## updated guide"));
            let version_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM doc_version WHERE doc_id=(SELECT id FROM doc WHERE repo_id=?1 AND slug='guide')",
                    params!["repo_merge"],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(version_count >= 1);
        }
    }

    #[test]
    fn import_docs_dedup_overwrite_skips_identical_content() {
        let db = test_db();
        {
            let conn = db.0.lock();
            insert_doc(&conn, "repo_dedupe", "guide", "Guide", "# Body", false);
        }
        {
            let conn = db.0.lock();
            let summary = import_docs_apply(
                &conn,
                vec![import_row("# Body", "identical")],
                Some("repo_dedupe".into()),
                None,
                false,
                "overwrite",
                "import.json",
                None,
            )
            .unwrap();
            assert_eq!(summary["skipped"].as_u64().unwrap(), 1);
            let version_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM doc_version WHERE doc_id=(SELECT id FROM doc WHERE repo_id=?1 AND slug='guide')",
                    params!["repo_dedupe"],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(version_count, 1);
        }
    }

    #[test]
    fn import_docs_writes_progress_file() {
        let db = test_db();
        let docs = vec![DocImportRow {
            id: None,
            repo_id: None,
            slug: "progress-doc".into(),
            title: Some("Progress".into()),
            body: Some("# Body".into()),
            is_deleted: Some(false),
            updated_at: None,
            versions: None,
        }];
        let progress_path = std::env::temp_dir()
            .join(format!("ae-import-progress-{}.log", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .to_string();
        {
            let conn = db.0.lock();
            let _ = import_docs_apply(
                &conn,
                docs,
                None,
                Some("Progress Repo".into()),
                false,
                "keep",
                "import.json",
                Some(progress_path.as_str()),
            )
            .unwrap();
        }
        let content = std::fs::read_to_string(&progress_path).unwrap();
        assert!(content.contains("\"status\":\"imported\""));
        let _ = std::fs::remove_file(&progress_path);
    }

    #[test]
    fn import_docs_imports_attachments() {
        let db = test_db();
        let tar_path = std::env::temp_dir().join(format!("ae-import-attachments-{}.tar", uuid::Uuid::new_v4()));
        let doc_id = uuid::Uuid::new_v4().to_string();
        let slug = "Guide Attachments";
        let slug_key = format!("{}-{}", sanitize_slug_for_filename(slug), doc_id);
        let repo_name = "Attachments Repo";
        {
            let file = File::create(&tar_path).unwrap();
            let mut builder = Builder::new(file);
            let docs_payload = serde_json::json!([{
                "id": doc_id,
                "repo_id": "repo_attach",
                "slug": slug,
                "title": "Guide Attachments",
                "body": "# Body",
                "is_deleted": false
            }]);
            append_tar_bytes(&mut builder, "docs.json", serde_json::to_vec(&docs_payload).unwrap().as_slice());
            append_tar_bytes(
                &mut builder,
                "meta.json",
                serde_json::to_vec(&serde_json::json!({"doc_count": 1, "format": "json"})).unwrap().as_slice(),
            );
            let doc_md_path = format!("docs/{}.md", slug_key);
            append_tar_bytes(&mut builder, &doc_md_path, b"# Body");
            let attachment_path = format!("attachments/{}/logo.png", slug_key);
            append_tar_bytes(&mut builder, &attachment_path, b"\x89PNGdata");
            builder.finish().unwrap();
        }
        let docs = read_docs_from_tar(&tar_path).unwrap();
        assert_eq!(docs[0].attachments.as_ref().map(|v| v.len()), Some(1));
        {
            let conn = db.0.lock();
            let _ = import_docs_apply(&conn, docs, None, Some(repo_name.into()), false, "keep", tar_path.to_string_lossy().as_ref(), None).unwrap();
        }
        {
            let conn = db.0.lock();
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM doc_asset", [], |r| r.get(0)).unwrap();
            assert_eq!(count, 1);
            let mime: String = conn.query_row("SELECT mime FROM doc_asset LIMIT 1", [], |r| r.get(0)).unwrap();
            assert_eq!(mime, "image/png");
        }
    }
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
    let version_hash = doc_version_hash(&doc_id, &payload.body);
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
    let version_hash = doc_version_hash(&payload.doc_id, &payload.body);
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
pub struct DocVersionExport {
    pub id: String,
    pub doc_id: String,
    pub created_at: String,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct DocExportRow {
    pub id: String,
    pub repo_id: String,
    pub slug: String,
    pub title: String,
    pub body: String,
    pub updated_at: String,
    pub is_deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<Vec<DocVersionExport>>,
}

fn export_docs_sql(include_deleted: bool, with_repo: bool) -> String {
    let mut sql = String::from(
        "SELECT d.id, d.repo_id, d.slug, d.title, COALESCE(fts.body,'') as body, d.updated_at, d.is_deleted \
         FROM doc d JOIN doc_fts fts ON fts.rowid = d.rowid WHERE 1=1",
    );
    if !include_deleted {
        sql.push_str(" AND d.is_deleted=0");
    }
    if with_repo {
        sql.push_str(" AND d.repo_id=?1");
    }
    sql.push_str(" ORDER BY d.updated_at DESC");
    sql
}

fn fetch_doc_exports(conn: &Connection, repo_id: Option<&str>, include_deleted: bool) -> Result<Vec<DocExportRow>, String> {
    let mut out = Vec::new();
    let sql = export_docs_sql(include_deleted, repo_id.is_some());
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = if let Some(repo) = repo_id {
        stmt.query_map(params![repo], |r| {
            Ok(DocExportRow {
                id: r.get(0)?,
                repo_id: r.get(1)?,
                slug: r.get(2)?,
                title: r.get(3)?,
                body: r.get(4)?,
                updated_at: r.get(5)?,
                is_deleted: r.get::<_, i64>(6)? != 0,
                versions: None,
            })
        })
    } else {
        stmt.query_map([], |r| {
            Ok(DocExportRow {
                id: r.get(0)?,
                repo_id: r.get(1)?,
                slug: r.get(2)?,
                title: r.get(3)?,
                body: r.get(4)?,
                updated_at: r.get(5)?,
                is_deleted: r.get::<_, i64>(6)? != 0,
                versions: None,
            })
        })
    };
    let rows = rows.map_err(|e| e.to_string())?;
    for row in rows { out.push(row.map_err(|e| e.to_string())?) }
    Ok(out)
}

fn fetch_doc_versions(conn: &Connection, doc_ids: &[String]) -> Result<HashMap<String, Vec<DocVersionExport>>, String> {
    if doc_ids.is_empty() { return Ok(HashMap::new()); }
    let mut map: HashMap<String, Vec<DocVersionExport>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT id, doc_id, created_at, hash, message FROM doc_version WHERE doc_id IN (SELECT value FROM json_each(?1)) ORDER BY created_at",
    ).map_err(|e| e.to_string())?;
    let ids_json = serde_json::to_string(doc_ids).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![ids_json], |r| {
            Ok(DocVersionExport {
                id: r.get(0)?,
                doc_id: r.get(1)?,
                created_at: r.get(2)?,
                hash: r.get(3)?,
                message: r.get::<_, Option<String>>(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let item = row.map_err(|e| e.to_string())?;
        map.entry(item.doc_id.clone()).or_default().push(item);
    }
    Ok(map)
}

#[tauri::command]
pub async fn export_docs(repo_id: Option<String>, include_deleted: Option<bool>, include_versions: Option<bool>, db: State<'_, std::sync::Arc<Db>>) -> Result<Vec<DocExportRow>, String> {
    let include_deleted = include_deleted.unwrap_or(false);
    let include_versions = include_versions.unwrap_or(false);
    let conn = db.0.lock();
    let mut docs = fetch_doc_exports(&conn, repo_id.as_deref(), include_deleted)?;
    if include_versions && !docs.is_empty() {
        let ids: Vec<String> = docs.iter().map(|d| d.id.clone()).collect();
        let version_map = fetch_doc_versions(&conn, &ids)?;
        for doc in docs.iter_mut() {
            if let Some(list) = version_map.get(&doc.id) {
                doc.versions = Some(list.clone());
            }
        }
    }
    Ok(docs)
}

fn read_docs_from_json(path: &Path) -> Result<Vec<DocImportRow>, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&data).map_err(|e| e.to_string())
}

fn read_docs_from_jsonl(path: &Path) -> Result<Vec<DocImportRow>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut docs = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() { continue; }
        let doc: DocImportRow = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        docs.push(doc);
    }
    Ok(docs)
}

fn sanitize_slug_for_filename(slug: &str) -> String {
    let mut sanitized: String = slug
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    if sanitized.is_empty() {
        sanitized = "doc".into();
    } else {
        sanitized = sanitized.to_lowercase();
    }
    if sanitized.len() > 40 {
        sanitized.truncate(40);
    }
    sanitized
}

fn parse_doc_filename_keys(stem: &str) -> (Option<String>, Option<String>) {
    if let Some(idx) = stem.rfind('-') {
        let candidate = &stem[idx + 1..];
        if candidate.len() == 36 && Uuid::parse_str(candidate).is_ok() {
            let slug_part = stem[..idx].to_string();
            let slug_key = if slug_part.is_empty() { None } else { Some(slug_part) };
            return (Some(candidate.to_string()), slug_key);
        }
    }
    (None, if stem.is_empty() { None } else { Some(stem.to_string()) })
}

struct PendingAttachment {
    id_key: Option<String>,
    slug_key: String,
    attachment: DocAttachmentImport,
}

fn read_docs_from_tar(path: &Path) -> Result<Vec<DocImportRow>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut archive = Archive::new(file);
    let mut docs_buf = Vec::new();
    let mut versions_buf = Vec::new();
    let mut pending_attachments: Vec<PendingAttachment> = Vec::new();
    let mut body_by_id: HashMap<String, String> = HashMap::new();
    let mut body_by_slug: HashMap<String, String> = HashMap::new();
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let name = path.to_string_lossy().to_string();
        if name == "docs.json" {
            entry.read_to_end(&mut docs_buf).map_err(|e| e.to_string())?;
        } else if name == "versions.json" {
            entry.read_to_end(&mut versions_buf).map_err(|e| e.to_string())?;
        } else if name.starts_with("attachments/") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            if buf.is_empty() {
                continue;
            }
            let rel = name.trim_start_matches("attachments/");
            let mut parts = rel.splitn(2, '/');
            let ident = parts.next().unwrap_or("").to_string();
            let filename = parts.next().unwrap_or("").to_string();
            if ident.is_empty() || filename.is_empty() {
                continue;
            }
            let (doc_id_key, slug_hint) = parse_doc_filename_keys(&ident);
            let slug_key = slug_hint.unwrap_or_else(|| ident.clone());
            pending_attachments.push(PendingAttachment {
                id_key: doc_id_key,
                slug_key,
                attachment: DocAttachmentImport {
                    filename,
                    mime: None,
                    encoding: Some("binary".into()),
                    data_base64: None,
                    bytes: buf,
                },
            });
        } else if name.starts_with("docs/") && name.ends_with(".md") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            if buf.is_empty() {
                continue;
            }
            let body = String::from_utf8(buf).map_err(|e| e.to_string())?;
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if stem.is_empty() {
                continue;
            }
            let (doc_id_key, slug_key_hint) = parse_doc_filename_keys(&stem);
            if let Some(doc_id) = doc_id_key {
                body_by_id.insert(doc_id, body.clone());
            }
            let slug_key = slug_key_hint.unwrap_or(stem);
            if !slug_key.is_empty() {
                body_by_slug.insert(slug_key, body);
            }
        }
    }
    if docs_buf.is_empty() {
        return Err("docs.json missing from archive".into());
    }
    let mut docs: Vec<DocImportRow> = serde_json::from_slice(&docs_buf).map_err(|e| e.to_string())?;
    if !versions_buf.is_empty() {
        #[derive(Deserialize)]
        struct VersionBundle { doc_id: String, versions: Vec<DocVersionImport> }
        let bundles: Vec<VersionBundle> = serde_json::from_slice(&versions_buf).map_err(|e| e.to_string())?;
        let mut map: HashMap<String, Vec<DocVersionImport>> = HashMap::new();
        for b in bundles { map.insert(b.doc_id, b.versions); }
        for doc in docs.iter_mut() {
            if let Some(vs) = map.remove(doc.id.as_deref().unwrap_or("")) {
                doc.versions = Some(vs);
            }
        }
    }
    let mut doc_id_index: HashMap<String, usize> = HashMap::new();
    let mut doc_slug_index: HashMap<String, usize> = HashMap::new();
    for (idx, doc) in docs.iter_mut().enumerate() {
        if let Some(ref doc_id) = doc.id {
            doc_id_index.insert(doc_id.clone(), idx);
        }
        doc_slug_index.insert(sanitize_slug_for_filename(&doc.slug), idx);
        let has_body = doc.body.as_ref().map(|b| !b.is_empty()).unwrap_or(false);
        if has_body {
            continue;
        }
        if let Some(ref doc_id) = doc.id {
            if let Some(body) = body_by_id.remove(doc_id) {
                doc.body = Some(body);
                continue;
            }
        }
        let slug_key = sanitize_slug_for_filename(&doc.slug);
        if let Some(body) = body_by_slug.remove(&slug_key) {
            doc.body = Some(body);
            continue;
        }
        return Err(format!("doc {} missing body in docs.json and docs/*.md", doc.slug));
    }
    for pending in pending_attachments {
        if let Some(ref doc_id) = pending.id_key {
            if let Some(&idx) = doc_id_index.get(doc_id) {
                docs[idx].attachments.get_or_insert_with(Vec::new).push(pending.attachment);
                continue;
            }
        }
        if let Some(&idx) = doc_slug_index.get(&pending.slug_key) {
            docs[idx].attachments.get_or_insert_with(Vec::new).push(pending.attachment);
            continue;
        }
        return Err(format!("attachment for key {} not matched to doc", pending.slug_key));
    }
    Ok(docs)
}

fn read_docs_from_path(path: &Path) -> Result<Vec<DocImportRow>, String> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "json" => read_docs_from_json(path),
        "jsonl" => read_docs_from_jsonl(path),
        "tar" | "tgz" => read_docs_from_tar(path),
        _ => read_docs_from_json(path),
    }
}

fn import_docs_apply(
    conn: &Connection,
    docs: Vec<DocImportRow>,
    repo_id: Option<String>,
    new_repo_name: Option<String>,
    dry_run: bool,
    merge_strategy: &str,
    path: &str,
    progress_path: Option<&str>,
) -> Result<serde_json::Value, String> {
    if docs.is_empty() {
        emit_import_progress(progress_path, 0, 0, &ImportStats::default(), if dry_run { "dry_run" } else { "imported" }).ok();
        return Ok(serde_json::json!({
            "path": path,
            "doc_count": 0,
            "inserted": 0,
            "updated": 0,
            "skipped": 0,
            "dry_run": dry_run,
            "merge_strategy": merge_strategy,
            "status": if dry_run { "dry_run" } else { "imported" },
        }));
    }
    if merge_strategy != "keep" && merge_strategy != "overwrite" {
        return Err("merge_strategy must be keep or overwrite".into());
    }

    if repo_id.is_none() && new_repo_name.is_none() {
        return Err("specify repo_id or new_repo_name".into());
    }

    let doc_count = docs.len();
    if dry_run {
        let (target_repo, _) = resolve_repo_for_import(conn, repo_id.clone(), new_repo_name.clone(), true)?;
        let stats = simulate_import(conn, &docs, &target_repo, merge_strategy, repo_id.is_none())?;
        emit_import_progress(progress_path, doc_count, doc_count, &stats, "dry_run").ok();
        return Ok(serde_json::json!({
            "path": path,
            "doc_count": doc_count,
            "repo_id": target_repo,
            "dry_run": true,
            "merge_strategy": merge_strategy,
            "inserted": stats.inserted,
            "updated": stats.updated,
            "skipped": stats.skipped,
            "status": "dry_run",
        }));
    }

    let (target_repo, created_repo) = resolve_repo_for_import(conn, repo_id.clone(), new_repo_name.clone(), false)?;
    let folder_id = ensure_root_folder(conn, &target_repo)?;
    let mut stats = ImportStats::default();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (idx, doc) in docs.into_iter().enumerate() {
        import_doc_record(&tx, &target_repo, &folder_id, doc, merge_strategy, &mut stats, path)?;
        let processed = idx + 1;
        if processed % IMPORT_PROGRESS_INTERVAL == 0 || processed == doc_count {
            emit_import_progress(progress_path, processed, doc_count, &stats, "processing")?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    emit_import_progress(progress_path, doc_count, doc_count, &stats, "imported").ok();
    Ok(serde_json::json!({
        "path": path,
        "doc_count": doc_count,
        "repo_id": target_repo,
        "created_repo": created_repo,
        "dry_run": false,
        "merge_strategy": merge_strategy,
        "inserted": stats.inserted,
        "updated": stats.updated,
        "skipped": stats.skipped,
        "status": "imported",
    }))
}

fn simulate_import(conn: &Connection, docs: &[DocImportRow], repo_id: &str, merge_strategy: &str, repo_is_new: bool) -> Result<ImportStats, String> {
    let mut stats = ImportStats::default();
    if repo_is_new {
        stats.inserted = docs.len() as u32;
        return Ok(stats);
    }
    for doc in docs {
        let slug = doc.slug.trim();
        if slug.is_empty() {
            return Err("doc slug is required".into());
        }
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM doc WHERE repo_id=?1 AND slug=?2",
                params![repo_id, slug],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match exists {
            Some(_) => {
                if merge_strategy == "keep" {
                    stats.skipped += 1;
                } else {
                    stats.updated += 1;
                }
            }
            None => stats.inserted += 1,
        }
    }
    Ok(stats)
}

fn import_doc_record(
    conn: &Connection,
    repo_id: &str,
    folder_id: &str,
    mut doc: DocImportRow,
    merge_strategy: &str,
    stats: &mut ImportStats,
    import_path: &str,
) -> Result<(), String> {
    let slug = doc.slug.trim().to_string();
    if slug.is_empty() {
        return Err("doc slug is required".into());
    }
    let body = doc.body.take().ok_or_else(|| format!("doc {slug} missing body"))?;
    let title = doc.title.take().unwrap_or_else(|| slug.clone());
    let is_deleted = doc.is_deleted.unwrap_or(false);
    let message = doc
        .versions
        .as_ref()
        .and_then(|v| v.last())
        .and_then(|v| v.message.clone());
    let attachments = doc.attachments.take();

    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM doc WHERE repo_id=?1 AND slug=?2",
            params![repo_id, slug.clone()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    match existing {
        Some(doc_id) => {
            if merge_strategy == "keep" {
                stats.skipped += 1;
                return Ok(());
            }
            if content_matches_current(conn, &doc_id, &body)? {
                import_doc_attachments(conn, &doc_id, attachments)?;
                stats.skipped += 1;
                return Ok(());
            }
            update_doc_record(conn, &doc_id, repo_id, &slug, &title, &body, is_deleted, message.as_deref(), import_path)?;
            import_doc_attachments(conn, &doc_id, attachments)?;
            stats.updated += 1;
        }
        None => {
            let doc_id = doc.id.unwrap_or_else(|| Uuid::new_v4().to_string());
            insert_doc_record(conn, &doc_id, repo_id, folder_id, &slug, &title, &body, is_deleted, message.as_deref(), import_path)?;
            import_doc_attachments(conn, &doc_id, attachments)?;
            stats.inserted += 1;
        }
    }
    Ok(())
}

fn insert_doc_record(
    conn: &Connection,
    doc_id: &str,
    repo_id: &str,
    folder_id: &str,
    slug: &str,
    title: &str,
    body: &str,
    is_deleted: bool,
    message: Option<&str>,
    import_path: &str,
) -> Result<(), String> {
    let size_bytes = body.len() as i64;
    let line_count = body.lines().count() as i64;
    conn.execute(
        "INSERT INTO doc(id,repo_id,folder_id,slug,title,is_deleted,size_bytes,line_count,created_at,updated_at) VALUES(?,?,?,?,?,?,?, ?,datetime('now'),datetime('now'))",
        params![doc_id, repo_id, folder_id, slug, title, if is_deleted { 1 } else { 0 }, size_bytes, line_count],
    )
    .map_err(|e| e.to_string())?;
    write_doc_version(conn, doc_id, body, message)?;
    refresh_doc_fts(conn, doc_id, title, body, slug, repo_id)?;
    crate::graph::update_links_for_doc(conn, doc_id, body)?;
    record_import_provenance(conn, doc_id, import_path)?;
    Ok(())
}

fn update_doc_record(
    conn: &Connection,
    doc_id: &str,
    repo_id: &str,
    slug: &str,
    title: &str,
    body: &str,
    is_deleted: bool,
    message: Option<&str>,
    import_path: &str,
) -> Result<(), String> {
    let size_bytes = body.len() as i64;
    let line_count = body.lines().count() as i64;
    conn.execute(
        "UPDATE doc SET title=?1, is_deleted=?2, size_bytes=?3, line_count=?4, updated_at=datetime('now') WHERE id=?5",
        params![title, if is_deleted { 1 } else { 0 }, size_bytes, line_count, doc_id],
    )
    .map_err(|e| e.to_string())?;
    write_doc_version(conn, doc_id, body, message)?;
    refresh_doc_fts(conn, doc_id, title, body, slug, repo_id)?;
    crate::graph::update_links_for_doc(conn, doc_id, body)?;
    record_import_provenance(conn, doc_id, import_path)?;
    Ok(())
}

fn insert_doc_blob(conn: &Connection, content: &[u8], encoding: Option<&str>, mime: Option<&str>) -> Result<String, String> {
    let blob_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO doc_blob(id,content,size_bytes,encoding,mime) VALUES(?,?,?,?,?)",
        params![blob_id, content, content.len() as i64, encoding.unwrap_or("utf8"), mime.unwrap_or("text/markdown")],
    )
    .map_err(|e| e.to_string())?;
    Ok(blob_id)
}

fn doc_version_hash(doc_id: &str, body: &str) -> String {
    let body_hash = blake3::hash(body.as_bytes()).to_hex().to_string();
    format!("{doc_id}:{body_hash}")
}

fn content_matches_current(conn: &Connection, doc_id: &str, body: &str) -> Result<bool, String> {
    let new_hash = doc_version_hash(doc_id, body);
    let existing: Option<String> = conn
        .query_row(
            "SELECT v.hash FROM doc d JOIN doc_version v ON v.id=d.current_version_id WHERE d.id=?1",
            params![doc_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(existing.map(|h| h == new_hash).unwrap_or(false))
}

fn write_doc_version(conn: &Connection, doc_id: &str, body: &str, message: Option<&str>) -> Result<(), String> {
    let blob_id = insert_doc_blob(conn, body.as_bytes(), Some("utf8"), Some("text/markdown"))?;
    let hash = doc_version_hash(doc_id, body);
    let version_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO doc_version(id,doc_id,blob_id,hash,message,created_at) VALUES(?,?,?,?,?,datetime('now'))",
        params![version_id, doc_id, blob_id, hash, message.unwrap_or("")],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE doc SET current_version_id=?1 WHERE id=?2",
        params![version_id, doc_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn refresh_doc_fts(conn: &Connection, doc_id: &str, title: &str, body: &str, slug: &str, repo_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO doc_fts(doc_fts,rowid) VALUES('delete',(SELECT rowid FROM doc WHERE id=?1))",
        params![doc_id],
    )
    .ok();
    conn.execute(
        "INSERT INTO doc_fts(rowid,title,body,slug,repo_id) SELECT d.rowid,?2,?3,?4,?5 FROM doc d WHERE d.id=?1",
        params![doc_id, title, body, slug, repo_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn record_import_provenance(conn: &Connection, doc_id: &str, path: &str) -> Result<(), String> {
    let meta = serde_json::json!({ "path": path });
    conn.execute(
        "INSERT INTO provenance(id,entity_type,entity_id,source,meta,created_at) VALUES(?,?,?,?,?,datetime('now'))",
        params![Uuid::new_v4().to_string(), "doc", doc_id, "import", meta.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn infer_mime_from_filename(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn import_doc_attachments(conn: &Connection, doc_id: &str, attachments: Option<Vec<DocAttachmentImport>>) -> Result<(), String> {
    let Some(mut list) = attachments else { return Ok(()); };
    for mut attachment in list.drain(..) {
        let bytes = attachment.take_bytes()?;
        let encoding = attachment.encoding.as_deref().unwrap_or("binary");
        let mime = attachment.mime.as_deref().unwrap_or_else(|| infer_mime_from_filename(&attachment.filename));
        let blob_id = insert_doc_blob(conn, &bytes, Some(encoding), Some(mime))?;
        let asset_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO doc_asset(id,doc_id,filename,mime,size_bytes,blob_id,created_at) VALUES(?,?,?,?,?,?,datetime('now'))",
            params![asset_id, doc_id, attachment.filename, mime, bytes.len() as i64, blob_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_root_folder(conn: &Connection, repo_id: &str) -> Result<String, String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM folder WHERE repo_id=?1 ORDER BY created_at LIMIT 1",
            params![repo_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let folder_id = Uuid::new_v4().to_string();
    let path = format!("/{repo_id}");
    let slug = format!("root-{repo_id}");
    conn.execute(
        "INSERT INTO folder(id,repo_id,parent_id,path,slug,created_at,updated_at) VALUES(?1,?2,NULL,?3,?4,datetime('now'),datetime('now'))",
        params![folder_id, repo_id, path, slug],
    )
    .map_err(|e| e.to_string())?;
    Ok(folder_id)
}

fn resolve_repo_for_import(conn: &Connection, repo_id: Option<String>, new_repo_name: Option<String>, dry_run: bool) -> Result<(String, bool), String> {
    if let Some(id) = repo_id {
        let exists: Option<String> = conn
            .query_row("SELECT id FROM repo WHERE id=?1", params![&id], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        if exists.is_none() {
            return Err("repo not found".into());
        }
        return Ok((id, false));
    }
    if let Some(name) = new_repo_name {
        if dry_run {
            return Ok((format!("pending:{}", name.replace(' ', "-")), true));
        }
        let repo_id = Uuid::new_v4().to_string();
        let path = format!(".import/{}", name.replace(' ', "-"));
        conn.execute(
            "INSERT INTO repo(id,name,path,settings,created_at,updated_at) VALUES(?,?,?,json('{}'),datetime('now'),datetime('now'))",
            params![repo_id, name, path],
        )
        .map_err(|e| e.to_string())?;
        ensure_root_folder(conn, &repo_id)?;
        return Ok((repo_id, true));
    }
    Err("specify repo_id or new_repo_name".into())
}

#[tauri::command]
pub async fn export_db(out_path: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let dest = PathBuf::from(out_path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = db.0.lock();
    let mut dest_conn = Connection::open(&dest).map_err(|e| e.to_string())?;
    {
        let mut backup = Backup::new(&*conn, "main", &mut dest_conn, "main").map_err(|e| e.to_string())?;
        backup.step(-1).map_err(|e| e.to_string())?;
        backup.finish().map_err(|e| e.to_string())?;
    }
    dest_conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", []).ok();
    let bytes = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    Ok(serde_json::json!({
        "path": dest.to_string_lossy(),
        "bytes": bytes,
    }))
}

#[derive(Deserialize)]
pub struct ImportDocsPayload {
    pub path: String,
    pub repo_id: Option<String>,
    pub new_repo_name: Option<String>,
    pub dry_run: Option<bool>,
    pub merge_strategy: Option<String>,
    pub progress_path: Option<String>,
}

#[derive(Default)]
struct ImportStats {
    inserted: u32,
    updated: u32,
    skipped: u32,
}

const IMPORT_PROGRESS_INTERVAL: usize = 25;

#[derive(Serialize)]
struct ImportProgressEvent {
    status: String,
    processed: usize,
    total: usize,
    inserted: u32,
    updated: u32,
    skipped: u32,
}

fn emit_import_progress(progress_path: Option<&str>, processed: usize, total: usize, stats: &ImportStats, status: &str) -> Result<(), String> {
    let Some(path) = progress_path else { return Ok(()); };
    let event = ImportProgressEvent {
        status: status.into(),
        processed,
        total,
        inserted: stats.inserted,
        updated: stats.updated,
        skipped: stats.skipped,
    };
    let mut file = OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    let line = serde_json::to_string(&event).map_err(|e| e.to_string())?;
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn import_docs_exec(db: &std::sync::Arc<Db>, payload: ImportDocsPayload) -> Result<serde_json::Value, String> {
    let ImportDocsPayload { path, repo_id, new_repo_name, dry_run, merge_strategy, progress_path } = payload;
    if repo_id.is_some() && new_repo_name.is_some() {
        return Err("repo_id and new_repo_name are mutually exclusive".into());
    }
    let docs = read_docs_from_path(Path::new(&path))?;
    let dry_run = dry_run.unwrap_or(true);
    let merge_strategy = merge_strategy.unwrap_or_else(|| "keep".to_string());
    let conn = db.0.lock();
    import_docs_apply(&conn, docs, repo_id, new_repo_name, dry_run, &merge_strategy, &path, progress_path.as_deref())
}

#[tauri::command]
pub async fn import_docs(payload: ImportDocsPayload, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    import_docs_exec(db.inner(), payload)
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
pub async fn ai_provider_resolve(doc_id: Option<String>, provider: Option<String>, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    // Resolve provider similar to ai_run_core
    let provider_name: String = {
        let conn = db.0.lock();
        let use_default = provider.as_deref().unwrap_or("default").is_empty() || provider.as_deref() == Some("default");
        if use_default {
            // Try repo default via doc_id if provided
            if let Some(ref did) = doc_id {
                let repo_id: Option<String> = conn
                    .query_row(
                        "SELECT d.repo_id FROM doc d WHERE d.id=?1 OR d.slug=?1",
                        rusqlite::params![did],
                        |r| r.get(0),
                    )
                    .ok();
                if let Some(rid) = repo_id {
                    let repo_default: Option<String> = conn
                        .query_row(
                            "SELECT json_extract(settings,'$.default_provider') FROM repo WHERE id=?1",
                            rusqlite::params![rid],
                            |r| r.get(0),
                        )
                        .ok();
                    if let Some(p) = repo_default.filter(|s: &String| !s.is_empty()) { p } else {
                        conn.query_row(
                            "SELECT value FROM app_setting WHERE key='default_provider'",
                            [],
                            |r| r.get::<_, String>(0),
                        )
                        .unwrap_or_else(|_| "local".into())
                    }
                } else {
                    conn.query_row(
                        "SELECT value FROM app_setting WHERE key='default_provider'",
                        [],
                        |r| r.get::<_, String>(0),
                    )
                    .unwrap_or_else(|_| "local".into())
                }
            } else {
                conn.query_row(
                    "SELECT value FROM app_setting WHERE key='default_provider'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "local".into())
            }
        } else {
            provider.unwrap()
        }
    };

    // Lookup kind/enabled and key presence
    let (kind, enabled) = {
        let conn = db.0.lock();
        conn
            .query_row(
                "SELECT kind, enabled FROM provider WHERE name=?1",
                rusqlite::params![&provider_name],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .unwrap_or_else(|_| (String::from("local"), 1))
    };

    let mut has_key = true;
    if kind == "remote" {
        has_key = secrets::provider_key_exists(&db, &provider_name)?;
    }
    let allowed = enabled != 0 && (kind != "remote" || has_key);
    Ok(serde_json::json!({
        "name": provider_name,
        "kind": kind,
        "enabled": enabled != 0,
        "has_key": has_key,
        "allowed": allowed
    }))
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
    let mut response_model = String::new();
    let response_text = if provider_name == "openrouter" {
        match crate::ai::call_openrouter(db, &req.prompt, &redacted) {
            Ok(res) => { response_model = res.model.clone(); res.text }
            Err(err) => format!("[openrouter:error:{}]\nPrompt: {}\n---\n{}", err, req.prompt, redacted),
        }
    } else {
        format!("[{}]\nPrompt: {}\n---\n{}", provider_name, req.prompt, redacted)
    };

    // Persist ai_trace
    let conn = db.0.lock();
    let trace_id = uuid::Uuid::new_v4().to_string();
    let request_json = serde_json::json!({"prompt": req.prompt, "context": redacted});
    let response_json = serde_json::json!({"text": response_text, "provider": provider_name, "model": response_model});
    conn.execute(
        "INSERT INTO ai_trace(id,repo_id,doc_id,anchor_id,provider,request,response,input_tokens,output_tokens,cost_usd) VALUES(?, (SELECT repo_id FROM doc WHERE id=?2 OR slug=?2), ?2, ?, ?, ?, ?, 0, 0, 0.0)",
        rusqlite::params![trace_id, req.doc_id, req.anchor_id.unwrap_or_default(), provider_name, request_json.to_string(), response_json.to_string()],
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"trace_id": trace_id, "text": response_text, "provider": provider_name, "model": response_model}))
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

// Provider model config helpers (OpenRouter and others)
#[tauri::command]
pub async fn ai_provider_model_get(name: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let model: Option<String> = conn
        .query_row(
            "SELECT json_extract(config,'$.model') FROM provider WHERE name=?1",
            params![name],
            |r| r.get(0),
        )
        .ok();
    Ok(serde_json::json!({"model": model.unwrap_or_default()}))
}

#[tauri::command]
pub async fn ai_provider_model_set(name: String, model: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "UPDATE provider SET config=json_set(COALESCE(config,json('{}')),'$.model',?2), updated_at=datetime('now') WHERE name=?1",
            params![name, model],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
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
struct CoreProc {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    exec: String,
    args: Vec<String>,
    restart_count: u32,
    max_restarts: u32,
    backoff_ms: u64,
}

fn plugin_log_line(name: &str, stream: &str, line: &str) {
    use std::fs::OpenOptions;
    use std::io::Write as _;
    let ts_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut msg = String::new();
    msg.push_str(&format!("[{}][plugin:{}][{}] ", ts_secs, name, stream));
    // Trim trailing newlines to keep log tidy
    let mut l = line.to_string();
    while l.ends_with('\n') || l.ends_with('\r') { l.pop(); }
    msg.push_str(&l);
    msg.push('\n');
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(".sidecar.log") {
        let _ = f.write_all(msg.as_bytes());
    }
}

fn spawn_core_child(name: &str, exec: &str, args: &Vec<String>) -> Result<(Child, Option<ChildStdin>, Option<ChildStdout>), String> {
    let mut cmd = OsCommand::new(exec);
    if !args.is_empty() { cmd.args(args); }
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    // Spawn stderr logger thread
    if let Some(stderr) = child.stderr.take() {
        let name_owned = name.to_string();
        std::thread::spawn(move || {
            let mut br = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match br.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => plugin_log_line(&name_owned, "stderr", &line),
                    Err(_) => break,
                }
            }
        });
    }
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    Ok((child, stdin, stdout))
}

#[tauri::command]
pub async fn plugins_spawn_core(name: String, exec: String, args: Option<Vec<String>>) -> Result<serde_json::Value, String> {
    static REG: OnceLock<Mutex<HashMap<String, CoreProc>>> = OnceLock::new();
    let reg = REG.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = reg.lock().map_err(|_| "lock_poison")?;
    if map.contains_key(&name) {
        return Err("already_running".into());
    }
    let args_v = args.unwrap_or_default();
    let (child, stdin, stdout) = spawn_core_child(&name, &exec, &args_v)?;
    let pid = child.id();
    map.insert(name.clone(), CoreProc {
        child,
        stdin,
        stdout,
        exec,
        args: args_v,
        restart_count: 0,
        max_restarts: 3,
        backoff_ms: 200,
    });
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

fn plugins_call_core_check(db: &std::sync::Arc<Db>, name: &str, line: &str) -> Result<(), String> {
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
    let parsed: serde_json::Value = serde_json::from_str(line).map_err(|_| "invalid_request".to_string())?;
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
    // net.request domain allowlist
    if method.starts_with("net.request") {
        let params_v = parsed.get("params").cloned().unwrap_or(serde_json::json!({}));
        let url_s = params_v.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if !url_s.is_empty() {
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
                                if host.eq_ignore_ascii_case(dom) || (dom.starts_with('.') && host.ends_with(dom)) { allowed = true; break; }
                            }
                        }
                    }
                }
            }
            if !allowed { return Err("forbidden_net_domain".into()); }
        }
    }
    // FS roots allowlist
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
    Ok(())
}

#[tauri::command]
pub async fn plugins_core_list() -> Result<Vec<serde_json::Value>, String> {
    static REG: OnceLock<Mutex<HashMap<String, CoreProc>>> = OnceLock::new();
    let reg = REG.get_or_init(|| Mutex::new(HashMap::new()));
    let mut out = Vec::new();
    if let Ok(mut map) = reg.lock() {
        for (name, proc) in map.iter_mut() {
            // Determine if still running
            let running = proc.child.try_wait().ok().flatten().is_none();
            let pid = proc.child.id();
            out.push(serde_json::json!({ "name": name, "pid": pid, "running": running }));
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn plugins_call_core(name: String, line: String, db: State<'_, std::sync::Arc<Db>>) -> Result<serde_json::Value, String> {
    plugins_call_core_check(&db, &name, &line)?;
    static REG: OnceLock<Mutex<HashMap<String, CoreProc>>> = OnceLock::new();
    let reg = REG.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = reg.lock().map_err(|_| "lock_poison")?;
    if let Some(proc) = map.get_mut(&name) {
        // Restart policy if process exited
        if proc.child.try_wait().ok().flatten().is_some() {
            if proc.restart_count < proc.max_restarts {
                std::thread::sleep(std::time::Duration::from_millis(proc.backoff_ms * (1 << proc.restart_count)));
                let (new_child, new_stdin, new_stdout) = spawn_core_child(&name, &proc.exec, &proc.args)?;
                proc.child = new_child;
                proc.stdin = new_stdin;
                proc.stdout = new_stdout;
                proc.restart_count += 1;
            } else {
                return Err("not_running".into());
            }
        }
        if let Some(stdin) = proc.stdin.as_mut() {
            stdin.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
            stdin.write_all(b"\n").map_err(|e| e.to_string())?;
            stdin.flush().ok();
        } else { return Err("stdin_closed".into()); }
        if let Some(stdout) = proc.stdout.take() {
            // Read one JSON line with a watchdog timeout and then return stdout to proc
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let mut reader = std::io::BufReader::new(stdout);
                let mut buf = String::new();
                let res = reader.read_line(&mut buf).map_err(|e| e.to_string());
                // Return both the (possibly updated) stdout and the line
                let stdout_back = reader.into_inner();
                let _ = tx.send((stdout_back, res, buf));
            });
            let timeout_ms: u64 = std::env::var("PLUGIN_CALL_TIMEOUT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(5000);
            match rx.recv_timeout(std::time::Duration::from_millis(timeout_ms)) {
                Ok((stdout_back, res, buf)) => {
                    proc.stdout = Some(stdout_back);
                    res.map_err(|e| e)?;
                    let trimmed = buf.trim();
                    if !trimmed.is_empty() { plugin_log_line(&name, "stdout", trimmed); }
                    if trimmed.is_empty() { return Ok(serde_json::json!({"ok": true})); }
                    let val: serde_json::Value = serde_json::from_str(trimmed).unwrap_or(serde_json::json!({"line": trimmed}));
                    return Ok(val);
                }
                Err(_timeout) => {
                    // On timeout, leave stdout as None and signal timeout; next call will trigger restart policy
                    return Err("timeout".into());
                }
            }
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
    use regex::Regex;
    let mut out = s.to_string();
    // AWS Access Key IDs (AKIA/ASIAxxxxxxxxxxxxxxxx)
    let re_aws_ak = Regex::new(r"(?i)\b(AKIA|ASIA)[0-9A-Z]{16}\b").unwrap();
    out = re_aws_ak.replace_all(&out, "****").to_string();

    // AWS Secret Access Key (40 chars base64-like)
    let re_aws_sk = Regex::new(r"(?i)(aws[_-]?secret[_-]?access[_-]?key\s*[:=]\s*['\"]?)([A-Za-z0-9/+=]{40})").unwrap();
    out = re_aws_sk.replace_all(&out, "$1****").to_string();

    // Bearer tokens
    let re_bearer = Regex::new(r"(?i)\b(bearer)\s+[A-Za-z0-9_\-\.]{16,}\b").unwrap();
    out = re_bearer.replace_all(&out, "$1 ****").to_string();

    // Generic api key/token param
    let re_key_param = Regex::new(r"(?i)(api[_-]?key|apikey|token|auth)_?id?\s*[:=]\s*['\"]?([A-Za-z0-9_\-]{16,})").unwrap();
    out = re_key_param.replace_all(&out, "$1=****").to_string();

    // URL query params ?key=..., &token=
    let re_query = Regex::new(r"([?&](?:key|api[_-]?key|token)=[^&\s]{4,})").unwrap();
    out = re_query.replace_all(&out, |caps: &regex::Captures| {
        let s = &caps[1];
        let k = s.split('=').next().unwrap_or("key");
        format!("{}=****", k)
    }).to_string();

    // High-entropy generic tokens: long hex/base64ish words (fallback)
    let re_entropy = Regex::new(r"\b[A-Za-z0-9/_\+=]{24,}\b").unwrap();
    out = re_entropy.replace_all(&out, |m: &regex::Captures| {
        let t = &m[0];
        // Avoid redacting typical prose by requiring mixed char classes
        let has_alpha = t.chars().any(|c| c.is_ascii_alphabetic());
        let has_digit = t.chars().any(|c| c.is_ascii_digit());
        if has_alpha && has_digit { "****".to_string() } else { t.to_string() }
    }).to_string();

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
//! Tauri IPC command handlers and helpers.
//!
//! This module exposes the app's capabilities to the UI via Tauri `#[command]` functions
//! and to the JSON-RPC sidecar (reusing the same core logic). Keep signatures small and
//! typed, release DB locks quickly, and return structured JSON or serializable structs.
//!
//! Groups:
//! - repos_*: add/list/info/remove/set_default_provider
//! - docs_*: create/update/get/delete; search and graph_* (neighbors/backlinks/related/path)
//! - ai_*: run, providers (list/enable/disable), keys, model get/set, resolve
//! - plugins_*: list/info/enable/disable/remove/upsert; spawn/shutdown/call; core_list
//! - anchors_*: upsert/list/delete
//!
//! Tests:
//! - Permission gating (plugins_call_core_check)
//! - Provider gating and missing key cases
//! - Redaction unit tests
//!
//! See docs: `docs/manual/RPC.md` and `docs/guides/CODEMAP.md`.
#[derive(Deserialize)]
struct DocVersionImport {
    id: Option<String>,
    hash: Option<String>,
    created_at: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct DocImportRow {
    id: Option<String>,
    repo_id: Option<String>,
    slug: String,
    title: Option<String>,
    body: Option<String>,
    is_deleted: Option<bool>,
    updated_at: Option<String>,
    versions: Option<Vec<DocVersionImport>>,
    #[serde(default)]
    attachments: Option<Vec<DocAttachmentImport>>,
}

#[derive(Deserialize)]
struct DocAttachmentImport {
    filename: String,
    mime: Option<String>,
    encoding: Option<String>,
    #[serde(default)]
    data_base64: Option<String>,
    #[serde(skip)]
    bytes: Vec<u8>,
}

impl DocAttachmentImport {
    fn take_bytes(&mut self) -> Result<Vec<u8>, String> {
        if !self.bytes.is_empty() {
            return Ok(std::mem::take(&mut self.bytes));
        }
        if let Some(data) = self.data_base64.take() {
            return STANDARD.decode(data).map_err(|e| e.to_string());
        }
        Err(format!("attachment {} missing data", self.filename))
    }
}
