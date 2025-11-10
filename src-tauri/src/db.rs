use parking_lot::Mutex;
use rusqlite::Connection;

pub struct Db(pub Mutex<Connection>);

pub fn open_db(path: &std::path::Path) -> Result<Db, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut conn = Connection::open(path)?;
    // Performance PRAGMAs
    conn.pragma_update(None, "journal_mode", &"WAL")?;
    conn.pragma_update(None, "synchronous", &"NORMAL")?;
    conn.pragma_update(None, "foreign_keys", &true)?;
    // DDL
    conn.execute_batch(include_str!("../schema.sql"))?;
    // Ensure app-controlled FTS updates: drop any leftover triggers that try to sync body from blobs
    let _ = conn.execute("DROP TRIGGER IF EXISTS doc_version_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS doc_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS doc_au", []);
    Ok(Db(Mutex::new(conn)))
}
