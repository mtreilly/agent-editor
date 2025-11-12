//! Graph query commands for document relationships

use crate::db::Db;
use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct GraphDoc {
    pub id: String,
    pub slug: String,
    pub title: String,
}

#[tauri::command]
pub async fn graph_backlinks(
    doc_id: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<GraphDoc>, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.slug, d.title FROM link l JOIN doc d ON d.id = l.from_doc_id WHERE l.to_doc_id = ?1 ORDER BY d.updated_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![doc_id], |r| {
            Ok(GraphDoc {
                id: r.get(0)?,
                slug: r.get(1)?,
                title: r.get(2)?,
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
pub async fn graph_neighbors(
    doc_id: String,
    depth: Option<u8>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<GraphDoc>, String> {
    // 1-hop for now per plan; can expand depth later
    let conn = db.0.lock();
    let _ = depth; // reserved for future use
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT d.id, d.slug, d.title FROM (
            SELECT l2.from_doc_id AS neighbor_id FROM link l
            JOIN link l2 ON l.to_doc_id = l2.to_doc_id
            WHERE l.from_doc_id = ?1 AND l2.from_doc_id != ?1
            UNION
            SELECT to_doc_id FROM link WHERE from_doc_id = ?1 AND to_doc_id IS NOT NULL
        ) n JOIN doc d ON d.id = n.neighbor_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![doc_id], |r| {
            Ok(GraphDoc {
                id: r.get(0)?,
                slug: r.get(1)?,
                title: r.get(2)?,
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
pub async fn graph_related(
    doc_id: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<GraphDoc>, String> {
    // Co-citation: docs that link to the same targets as doc_id
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare(
            "SELECT d2.id, d2.slug, d2.title, COUNT(*) as score
         FROM link l1
         JOIN link l2 ON l1.to_doc_id = l2.to_doc_id
         JOIN doc d2 ON d2.id = l2.from_doc_id
         WHERE l1.from_doc_id = ?1 AND l2.from_doc_id != ?1 AND l2.from_doc_id IS NOT NULL
         GROUP BY d2.id
         ORDER BY score DESC, d2.updated_at DESC
         LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![doc_id], |r| {
            Ok(GraphDoc {
                id: r.get(0)?,
                slug: r.get(1)?,
                title: r.get(2)?,
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
pub async fn graph_path(
    start_id: String,
    end_id: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<String>, String> {
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
    let path_json: Option<String> = stmt
        .query_row(params![start_id, end_id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(p) = path_json {
        let v: serde_json::Value = serde_json::from_str(&p).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        if let Some(arr) = v.as_array() {
            for x in arr {
                if let Some(s) = x.as_str() {
                    out.push(s.to_string())
                }
            }
        }
        return Ok(out);
    }
    Ok(vec![])
}
