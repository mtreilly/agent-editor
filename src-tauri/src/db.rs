//! Database layer: open and initialize SQLite, apply PRAGMAs, run schema DDL, and seed defaults.
//!
//! Responsibilities
//! - Open the app database at a known path (dev: `.dev/agent-editor.db` unless `AE_DB` env is set)
//! - Apply performance PRAGMAs (WAL, NORMAL sync) and enable foreign keys
//! - Execute `schema.sql` (kept alongside sources) to create/upgrade tables
//! - Seed provider rows with privacy-safe defaults (network off by default)
//!
//! See also:
//! - `commands.rs` for Tauri IPC commands using this DB
//! - `docs/manual/DATA_MODEL.md` for a high-level schema overview
//! - `docs/guides/SCANNER.md` for scanner → DB write pipeline

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
    // Seed providers (privacy defaults)
    seed_providers(&mut conn)?;
    // Ensure app-controlled FTS updates: drop any leftover triggers that try to sync body from blobs
    let _ = conn.execute("DROP TRIGGER IF EXISTS doc_version_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS doc_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS doc_au", []);
    Ok(Db(Mutex::new(conn)))
}

fn seed_providers(conn: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Insert defaults if missing
    let providers = vec![
        ("local", "local", 1),
        ("codex", "remote", 0),
        ("claude-code", "remote", 0),
        ("openrouter", "remote", 0),
        ("opencode", "remote", 0),
    ];
    for (name, kind, enabled) in providers {
        conn.execute(
            "INSERT INTO provider(name,kind,enabled,config) VALUES(?,?,?,json('{}')) ON CONFLICT(name) DO NOTHING",
            rusqlite::params![name, kind, enabled],
        )?;
    }
    Ok(())
}
