use rusqlite::{params, Connection, OptionalExtension};

pub fn update_links_for_doc(conn: &Connection, doc_id: &str, content: &str) -> Result<(), String> {
    // Get repo_id for resolving links
    let repo_id: String = conn
        .query_row("SELECT repo_id FROM doc WHERE id=?1", params![doc_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    // delete old links
    conn.execute("DELETE FROM link WHERE from_doc_id=?1", params![doc_id])
        .map_err(|e| e.to_string())?;

    for (to_slug, line_start, line_end) in extract_wikilinks(content) {
        // resolve to_doc_id if exists
        let to_id: Option<String> = conn
            .query_row(
                "SELECT id FROM doc WHERE repo_id=?1 AND slug=?2",
                params![repo_id, to_slug],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO link(id,repo_id,from_doc_id,to_doc_id,to_slug,type,line_start,line_end) VALUES(?,?,?,?,?,'wiki',?,?)",
            params![id, repo_id, doc_id, to_id, to_slug, line_start, line_end],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn extract_wikilinks(content: &str) -> Vec<(String, i64, i64)> {
    let mut res = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let mut s = line;
        while let Some(start) = s.find("[[") {
            if let Some(end) = s[start + 2..].find("]] ") { // spaced close unlikely; fallback below
                let inner = &s[start + 2..start + 2 + end];
                if let Some((slug, _alias)) = split_slug_alias(inner) {
                    let slug = slug_before_heading(slug);
                    res.push((slug, i as i64 + 1, i as i64 + 1));
                }
                s = &s[start + 2 + end + 2..];
                continue;
            }
            if let Some(end2) = s[start + 2..].find("]]" ) {
                let inner = &s[start + 2..start + 2 + end2];
                if let Some((slug, _alias)) = split_slug_alias(inner) {
                    let slug = slug_before_heading(slug);
                    res.push((slug, i as i64 + 1, i as i64 + 1));
                }
                s = &s[start + 2 + end2 + 2..];
            } else {
                break;
            }
        }
    }
    res
}

fn split_slug_alias(inner: &str) -> Option<(String, Option<String>)> {
    let parts: Vec<&str> = inner.split('|').collect();
    if parts.is_empty() { return None; }
    let slug = parts[0].trim().to_string();
    let alias = if parts.len() > 1 { Some(parts[1].trim().to_string()) } else { None };
    Some((slug, alias))
}

fn slug_before_heading(slug: String) -> String {
    match slug.split_once('#') { Some((s, _)) => s.to_string(), None => slug }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wikilinks_basic() {
        let md = "Line1 [[Alpha|A]] and [[Beta]]\nNext [[Gamma#Section]] end";
        let links = extract_wikilinks(md);
        // Expect three links with cleaned slugs and 1-based line numbers
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].0, "Alpha");
        assert_eq!(links[0].1, 1);
        assert_eq!(links[1].0, "Beta");
        assert_eq!(links[1].1, 1);
        assert_eq!(links[2].0, "Gamma");
        assert_eq!(links[2].1, 2);
    }
}
