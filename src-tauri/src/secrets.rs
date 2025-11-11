use rusqlite::params;
use crate::db::Db;

#[cfg(feature = "keyring")]
fn kr_entry(name: &str) -> keyring::Entry {
    let service = "agent-editor";
    keyring::Entry::new(service, name)
}

pub fn provider_key_set(db: &Db, name: &str, key: &str) -> Result<bool, String> {
    #[cfg(feature = "keyring")]
    {
        if let Err(e) = kr_entry(name).set_password(key) { return Err(format!("keyring_error: {}", e)); }
        return Ok(true)
    }
    // Fallback: store existence flag in DB provider.config (no secret material persisted ideally)
    let conn = db.0.lock();
    let n = conn.execute(
        "UPDATE provider SET config=json_set(COALESCE(config,json('{}')),'$.key_set',1), updated_at=datetime('now') WHERE name=?1",
        params![name],
    ).map_err(|e| e.to_string())?;
    let _ = key; // unused in fallback
    Ok(n > 0)
}

pub fn provider_key_exists(db: &Db, name: &str) -> Result<bool, String> {
    #[cfg(feature = "keyring")]
    {
        match kr_entry(name).get_password() { Ok(_) => return Ok(true), Err(_) => return Ok(false) }
    }
    let conn = db.0.lock();
    let has: i64 = conn.query_row(
        "SELECT COALESCE(json_extract(config,'$.key_set'),0) FROM provider WHERE name=?1",
        params![name],
        |r| r.get(0),
    ).unwrap_or(0);
    Ok(has != 0)
}

// Fetch provider secret from OS keychain when available.
// Without keyring feature enabled, return an explicit error to avoid leaking secrets via other channels.
pub fn provider_key_get(_db: &Db, name: &str) -> Result<String, String> {
    #[cfg(feature = "keyring")]
    {
        return kr_entry(name)
            .get_password()
            .map_err(|e| format!("keyring_error: {}", e));
    }
    let _ = name;
    Err("keyring_not_enabled".into())
}
