use crate::secrets;
use crate::db::Db;

pub fn provider_test(db: &Db, name: &str, prompt: &str) -> Result<serde_json::Value, String> {
    // Ensure key exists (for remote providers) and provider is enabled
    let conn = db.0.lock();
    let row: Option<(String, i64)> = conn
        .query_row(
            "SELECT kind, enabled FROM provider WHERE name=?1",
            rusqlite::params![name],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    drop(conn);
    if row.is_none() { return Err("not_found".into()); }
    let (kind, enabled) = row.unwrap();
    if enabled == 0 { return Err("disabled".into()); }
    if kind == "remote" && !secrets::provider_key_exists(db, name)? {
        return Err("no_key".into());
    }
    // Deterministic stub result
    Ok(serde_json::json!({ "provider": name, "ok": true, "echo": prompt }))
}

