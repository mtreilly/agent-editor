//! Document import/export commands

use crate::db::Db;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{backup::Backup, params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use tauri::State;
use uuid::Uuid;

// ===== Export Types =====

#[derive(Clone, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<DocAttachmentExport>>,
}

#[derive(Clone, Serialize)]
pub struct DocAttachmentExport {
    pub doc_id: String,
    pub filename: String,
    pub mime: String,
    pub encoding: String,
    pub data_base64: String,
}

// ===== Import Types =====

#[derive(Deserialize)]
pub struct ImportDocsPayload {
    pub path: String,
    pub repo_id: Option<String>,
    pub new_repo_name: Option<String>,
    pub dry_run: Option<bool>,
    pub merge_strategy: Option<String>,
    pub progress_path: Option<String>,
}

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

struct PendingAttachment {
    id_key: Option<String>,
    slug_key: String,
    attachment: DocAttachmentImport,
}

// ===== Export Functions =====

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

fn fetch_doc_exports(
    conn: &Connection,
    repo_id: Option<&str>,
    include_deleted: bool,
) -> Result<Vec<DocExportRow>, String> {
    let mut out = Vec::new();
    let sql = export_docs_sql(include_deleted, repo_id.is_some());
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let mapper = |r: &rusqlite::Row| -> rusqlite::Result<DocExportRow> {
        Ok(DocExportRow {
            id: r.get(0)?,
            repo_id: r.get(1)?,
            slug: r.get(2)?,
            title: r.get(3)?,
            body: r.get(4)?,
            updated_at: r.get(5)?,
            is_deleted: r.get::<_, i64>(6)? != 0,
            versions: None,
            attachments: None,
        })
    };

    let rows = if let Some(repo) = repo_id {
        stmt.query_map(params![repo], mapper).map_err(|e| e.to_string())?
    } else {
        stmt.query_map([], mapper).map_err(|e| e.to_string())?
    };

    for row in rows {
        out.push(row.map_err(|e| e.to_string())?)
    }
    Ok(out)
}

fn fetch_doc_versions(
    conn: &Connection,
    doc_ids: &[String],
) -> Result<HashMap<String, Vec<DocVersionExport>>, String> {
    if doc_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut map: HashMap<String, Vec<DocVersionExport>> = HashMap::new();
    let mut stmt = conn
        .prepare(
            "SELECT id, doc_id, created_at, hash, message FROM doc_version WHERE doc_id IN (SELECT value FROM json_each(?1)) ORDER BY created_at",
        )
        .map_err(|e| e.to_string())?;
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

fn fetch_doc_attachments(
    conn: &Connection,
    doc_ids: &[String],
) -> Result<HashMap<String, Vec<DocAttachmentExport>>, String> {
    if doc_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut map: HashMap<String, Vec<DocAttachmentExport>> = HashMap::new();
    let mut stmt = conn
        .prepare(
            "SELECT a.doc_id, a.filename, a.mime, b.encoding, b.content \
             FROM doc_asset a JOIN doc_blob b ON b.id = a.blob_id \
             WHERE a.doc_id IN (SELECT value FROM json_each(?1)) \
             ORDER BY a.created_at",
        )
        .map_err(|e| e.to_string())?;
    let ids_json = serde_json::to_string(doc_ids).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![ids_json], |r| {
            let doc_id: String = r.get(0)?;
            let filename: String = r.get(1)?;
            let mime: String = r.get(2)?;
            let encoding: String = r.get(3)?;
            let content: Vec<u8> = r.get(4)?;
            Ok((doc_id, filename, mime, encoding, content))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (doc_id, filename, mime, encoding, content) = row.map_err(|e| e.to_string())?;
        map.entry(doc_id.clone()).or_default().push(DocAttachmentExport {
            doc_id,
            filename,
            mime,
            encoding,
            data_base64: STANDARD.encode(content),
        });
    }
    Ok(map)
}

#[tauri::command]
pub async fn export_docs(
    repo_id: Option<String>,
    include_deleted: Option<bool>,
    include_versions: Option<bool>,
    include_attachments: Option<bool>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<DocExportRow>, String> {
    let include_deleted = include_deleted.unwrap_or(false);
    let include_versions = include_versions.unwrap_or(false);
    let include_attachments = include_attachments.unwrap_or(false);
    let conn = db.0.lock();
    let mut docs = fetch_doc_exports(&conn, repo_id.as_deref(), include_deleted)?;
    if include_versions && !docs.is_empty() {
        let ids: Vec<String> = docs.iter().map(|d| d.id.clone()).collect();
        let version_map = fetch_doc_versions(&conn, &ids)?;
        for doc in docs.iter_mut() {
            if let Some(list) = version_map.get(&doc.id) {
                doc.versions = Some(list.to_vec());
            }
        }
    }
    if include_attachments && !docs.is_empty() {
        let ids: Vec<String> = docs.iter().map(|d| d.id.clone()).collect();
        let attachment_map = fetch_doc_attachments(&conn, &ids)?;
        for doc in docs.iter_mut() {
            if let Some(list) = attachment_map.get(&doc.id) {
                doc.attachments = Some(list.to_vec());
            }
        }
    }
    Ok(docs)
}

#[tauri::command]
pub async fn export_db(
    out_path: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let dest = PathBuf::from(out_path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = db.0.lock();
    let mut dest_conn = Connection::open(&dest).map_err(|e| e.to_string())?;
    {
        let backup = Backup::new(&*conn, &mut dest_conn).map_err(|e| e.to_string())?;
        backup.step(-1).map_err(|e| e.to_string())?;
        // Backup is finalized automatically when dropped
    }
    dest_conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", []).ok();
    let bytes = std::fs::metadata(&dest)
        .map(|m| m.len())
        .unwrap_or(0);
    Ok(serde_json::json!({
        "path": dest.to_string_lossy(),
        "bytes": bytes,
    }))
}

// ===== Import Helper Functions =====

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
            let slug_key = if slug_part.is_empty() {
                None
            } else {
                Some(slug_part)
            };
            return (Some(candidate.to_string()), slug_key);
        }
    }
    (
        None,
        if stem.is_empty() {
            None
        } else {
            Some(stem.to_string())
        },
    )
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
        if line.trim().is_empty() {
            continue;
        }
        let doc: DocImportRow = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        docs.push(doc);
    }
    Ok(docs)
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
            entry
                .read_to_end(&mut docs_buf)
                .map_err(|e| e.to_string())?;
        } else if name == "versions.json" {
            entry
                .read_to_end(&mut versions_buf)
                .map_err(|e| e.to_string())?;
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
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
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
    let mut docs: Vec<DocImportRow> =
        serde_json::from_slice(&docs_buf).map_err(|e| e.to_string())?;
    if !versions_buf.is_empty() {
        #[derive(Deserialize)]
        struct VersionBundle {
            doc_id: String,
            versions: Vec<DocVersionImport>,
        }
        let bundles: Vec<VersionBundle> =
            serde_json::from_slice(&versions_buf).map_err(|e| e.to_string())?;
        let mut map: HashMap<String, Vec<DocVersionImport>> = HashMap::new();
        for b in bundles {
            map.insert(b.doc_id, b.versions);
        }
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
        return Err(format!(
            "doc {} missing body in docs.json and docs/*.md",
            doc.slug
        ));
    }
    for pending in pending_attachments {
        if let Some(ref doc_id) = pending.id_key {
            if let Some(&idx) = doc_id_index.get(doc_id) {
                docs[idx]
                    .attachments
                    .get_or_insert_with(Vec::new)
                    .push(pending.attachment);
                continue;
            }
        }
        if let Some(&idx) = doc_slug_index.get(&pending.slug_key) {
            docs[idx]
                .attachments
                .get_or_insert_with(Vec::new)
                .push(pending.attachment);
            continue;
        }
        return Err(format!(
            "attachment for key {} not matched to doc",
            pending.slug_key
        ));
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

/// Helper function to compute document version hash
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

fn insert_doc_blob(
    conn: &Connection,
    content: &[u8],
    encoding: Option<&str>,
    mime: Option<&str>,
) -> Result<String, String> {
    let blob_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO doc_blob(id,content,size_bytes,encoding,mime) VALUES(?,?,?,?,?)",
        params![
            blob_id,
            content,
            content.len() as i64,
            encoding.unwrap_or("utf8"),
            mime.unwrap_or("text/markdown")
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(blob_id)
}

fn write_doc_version(
    conn: &Connection,
    doc_id: &str,
    body: &str,
    message: Option<&str>,
) -> Result<(), String> {
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

fn refresh_doc_fts(
    conn: &Connection,
    doc_id: &str,
    title: &str,
    body: &str,
    slug: &str,
    repo_id: &str,
) -> Result<(), String> {
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
        params![
            Uuid::new_v4().to_string(),
            "doc",
            doc_id,
            "import",
            meta.to_string()
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn infer_mime_from_filename(filename: &str) -> &'static str {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
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

fn import_doc_attachments(
    conn: &Connection,
    doc_id: &str,
    attachments: Option<Vec<DocAttachmentImport>>,
) -> Result<(), String> {
    let Some(mut list) = attachments else {
        return Ok(());
    };
    for mut attachment in list.drain(..) {
        let bytes = attachment.take_bytes()?;
        let encoding = attachment.encoding.as_deref().unwrap_or("binary");
        let mime = attachment
            .mime
            .as_deref()
            .unwrap_or_else(|| infer_mime_from_filename(&attachment.filename));
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

fn ensure_root_folder(conn: &mut Connection, repo_id: &str) -> Result<String, String> {
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

fn resolve_repo_for_import(
    conn: &mut Connection,
    repo_id: Option<String>,
    new_repo_name: Option<String>,
    dry_run: bool,
) -> Result<(String, bool), String> {
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

fn emit_import_progress(
    progress_path: Option<&str>,
    processed: usize,
    total: usize,
    stats: &ImportStats,
    status: &str,
) -> Result<(), String> {
    let Some(path) = progress_path else {
        return Ok(());
    };
    let event = ImportProgressEvent {
        status: status.into(),
        processed,
        total,
        inserted: stats.inserted,
        updated: stats.updated,
        skipped: stats.skipped,
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let line = serde_json::to_string(&event).map_err(|e| e.to_string())?;
    file.write_all(line.as_bytes())
        .map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(())
}

fn simulate_import(
    conn: &Connection,
    docs: &[DocImportRow],
    repo_id: &str,
    merge_strategy: &str,
    repo_is_new: bool,
) -> Result<ImportStats, String> {
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
        params![
            doc_id,
            repo_id,
            folder_id,
            slug,
            title,
            if is_deleted { 1 } else { 0 },
            size_bytes,
            line_count
        ],
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
        params![
            title,
            if is_deleted { 1 } else { 0 },
            size_bytes,
            line_count,
            doc_id
        ],
    )
    .map_err(|e| e.to_string())?;
    write_doc_version(conn, doc_id, body, message)?;
    refresh_doc_fts(conn, doc_id, title, body, slug, repo_id)?;
    crate::graph::update_links_for_doc(conn, doc_id, body)?;
    record_import_provenance(conn, doc_id, import_path)?;
    Ok(())
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
    let body = doc
        .body
        .take()
        .ok_or_else(|| format!("doc {slug} missing body"))?;
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
            update_doc_record(
                conn,
                &doc_id,
                repo_id,
                &slug,
                &title,
                &body,
                is_deleted,
                message.as_deref(),
                import_path,
            )?;
            import_doc_attachments(conn, &doc_id, attachments)?;
            stats.updated += 1;
        }
        None => {
            let doc_id = doc.id.unwrap_or_else(|| Uuid::new_v4().to_string());
            insert_doc_record(
                conn,
                &doc_id,
                repo_id,
                folder_id,
                &slug,
                &title,
                &body,
                is_deleted,
                message.as_deref(),
                import_path,
            )?;
            import_doc_attachments(conn, &doc_id, attachments)?;
            stats.inserted += 1;
        }
    }
    Ok(())
}

fn import_docs_apply(
    conn: &mut Connection,
    docs: Vec<DocImportRow>,
    repo_id: Option<String>,
    new_repo_name: Option<String>,
    dry_run: bool,
    merge_strategy: &str,
    path: &str,
    progress_path: Option<&str>,
) -> Result<serde_json::Value, String> {
    if docs.is_empty() {
        emit_import_progress(
            progress_path,
            0,
            0,
            &ImportStats::default(),
            if dry_run { "dry_run" } else { "imported" },
        )
        .ok();
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
        let (target_repo, _) =
            resolve_repo_for_import(conn, repo_id.clone(), new_repo_name.clone(), true)?;
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

    let (target_repo, created_repo) =
        resolve_repo_for_import(conn, repo_id.clone(), new_repo_name.clone(), false)?;
    let folder_id = ensure_root_folder(conn, &target_repo)?;
    let mut stats = ImportStats::default();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (idx, doc) in docs.into_iter().enumerate() {
        import_doc_record(
            &tx,
            &target_repo,
            &folder_id,
            doc,
            merge_strategy,
            &mut stats,
            path,
        )?;
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

pub fn import_docs_exec(
    db: &std::sync::Arc<Db>,
    payload: ImportDocsPayload,
) -> Result<serde_json::Value, String> {
    let ImportDocsPayload {
        path,
        repo_id,
        new_repo_name,
        dry_run,
        merge_strategy,
        progress_path,
    } = payload;
    if repo_id.is_some() && new_repo_name.is_some() {
        return Err("repo_id and new_repo_name are mutually exclusive".into());
    }
    let docs = read_docs_from_path(Path::new(&path))?;
    let dry_run = dry_run.unwrap_or(true);
    let merge_strategy = merge_strategy.unwrap_or_else(|| "keep".to_string());
    let mut conn = db.0.lock();
    import_docs_apply(
        &mut *conn,
        docs,
        repo_id,
        new_repo_name,
        dry_run,
        &merge_strategy,
        &path,
        progress_path.as_deref(),
    )
}

#[tauri::command]
pub async fn import_docs(
    payload: ImportDocsPayload,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    import_docs_exec(db.inner(), payload)
}
