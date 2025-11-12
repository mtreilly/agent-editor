//! Full-text search commands

use crate::db::Db;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct SearchHit {
    pub id: String,
    pub slug: String,
    pub title_snip: String,
    pub body_snip: String,
    pub rank: f64,
}

#[tauri::command]
pub async fn search(
    repo_id: Option<String>,
    query: String,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<SearchHit>, String> {
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
    let rows = stmt
        .query_map(params![query, repo_id, lim, off], |r| {
            Ok(SearchHit {
                id: r.get(0)?,
                slug: r.get(1)?,
                rank: r.get::<_, f64>(2).unwrap_or(0.0),
                title_snip: r.get::<_, String>(3).unwrap_or_default(),
                body_snip: r.get::<_, String>(4).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?)
    }
    Ok(out)
}
