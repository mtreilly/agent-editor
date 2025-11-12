//! Plugin management commands

use crate::{db::Db, plugins};
use rusqlite::params;
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;

// ===== Plugin Database Commands =====

#[tauri::command]
pub async fn plugins_list(
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare("SELECT id,name,version,kind,enabled FROM plugin ORDER BY name ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, String>(0)?,
                "name": r.get::<_, String>(1)?,
                "version": r.get::<_, String>(2)?,
                "kind": r.get::<_, String>(3)?,
                "enabled": r.get::<_, i64>(4)? != 0
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?)
    }
    Ok(out)
}

#[tauri::command]
pub async fn plugins_enable(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute("UPDATE plugin SET enabled=1 WHERE name=?1", params![name])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn plugins_disable(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute("UPDATE plugin SET enabled=0 WHERE name=?1", params![name])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn plugins_info(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id,name,version,kind,manifest,permissions,enabled,installed_at FROM plugin WHERE name=?1",
        )
        .map_err(|e| e.to_string())?;
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
        }));
    }
    Err("not_found".into())
}

#[tauri::command]
pub async fn plugins_remove(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute("DELETE FROM plugin WHERE name=?1", params![name])
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"removed": n>0}))
}

#[tauri::command]
pub async fn plugins_upsert(
    name: String,
    kind: Option<String>,
    version: Option<String>,
    permissions: Option<String>,
    enabled: Option<bool>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let kind = kind.unwrap_or_else(|| "core".to_string());
    let version = version.unwrap_or_else(|| "dev".to_string());
    let perms = permissions.unwrap_or_else(|| "{}".to_string());
    let enabled = enabled.unwrap_or(true);
    let conn = db.0.lock();
    let n = conn
        .execute(
            "INSERT INTO plugin(id,name,version,kind,manifest,permissions,enabled) VALUES(?, ?, ?, ?, json('{}'), ?, ?) ON CONFLICT(name) DO UPDATE SET permissions=excluded.permissions, enabled=excluded.enabled, version=excluded.version, kind=excluded.kind",
            params![
                Uuid::new_v4().to_string(),
                name,
                version,
                kind,
                perms,
                if enabled { 1 } else { 0 }
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"upserted": n>0}))
}

// ===== Core Plugin Lifecycle Commands =====

#[tauri::command]
pub async fn plugins_spawn_core(
    name: String,
    exec: String,
    args: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    let spec = plugins::CorePluginSpec {
        name,
        exec,
        args: args.unwrap_or_default(),
        env: vec![],
        caps: plugins::Capabilities {
            fs_roots: vec![],
            net_domains: vec![],
            db_read: false,
            db_write: false,
            ai_providers: vec![],
        },
    };

    let _handle = plugins::spawn_core_plugin(&spec)?;

    // Get the list to find the PID
    let list = plugins::list_core_plugins();
    let pid = list
        .iter()
        .find(|(n, _, _)| n == &spec.name)
        .map(|(_, pid, _)| *pid)
        .unwrap_or(0);

    Ok(serde_json::json!({"pid": pid}))
}

#[tauri::command]
pub async fn plugins_shutdown_core(name: String) -> Result<serde_json::Value, String> {
    match plugins::shutdown_core_plugin(&name) {
        Ok(_) => Ok(serde_json::json!({"stopped": true})),
        Err(e) if e == "not_found" => Ok(serde_json::json!({"stopped": false})),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn plugins_core_list() -> Result<Vec<serde_json::Value>, String> {
    let list = plugins::list_core_plugins();
    let out: Vec<serde_json::Value> = list
        .into_iter()
        .map(|(name, pid, running)| {
            serde_json::json!({ "name": name, "pid": pid, "running": running })
        })
        .collect();
    Ok(out)
}

#[tauri::command]
pub async fn plugins_call_core(
    name: String,
    line: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    // Check capabilities first
    plugins_call_core_check(&db, &name, &line)?;

    // Get timeout from environment or use default
    let timeout_ms: u64 = std::env::var("PLUGIN_CALL_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    // Call plugin through the abstraction
    plugins::call_core_plugin_raw_with_timeout(
        &name,
        &line,
        std::time::Duration::from_millis(timeout_ms),
    )
}

// ===== Helper Functions =====

fn plugins_call_core_check(
    db: &std::sync::Arc<Db>,
    name: &str,
    line: &str,
) -> Result<(), String> {
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
        if allowed == 0 {
            return Err("forbidden".into());
        }
    }
    // Validate JSON-RPC envelope and method-level permissions
    let parsed: serde_json::Value =
        serde_json::from_str(line).map_err(|_| "invalid_request".to_string())?;
    let method = parsed
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    if method.is_empty() {
        return Err("invalid_request".into());
    }
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
        if allowed == 0 {
            return Err("forbidden".into());
        }
    }
    // net.request domain allowlist
    if method.starts_with("net.request") {
        let params_v = parsed.get("params").cloned().unwrap_or(serde_json::json!({}));
        let url_s = params_v.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if !url_s.is_empty() {
            let host = if let Some(rest) = url_s.split("//").nth(1) {
                rest.split('/')
                    .next()
                    .unwrap_or("")
                    .split(':')
                    .next()
                    .unwrap_or("")
            } else {
                url_s
            };
            let conn = db.0.lock();
            let perms_json: Option<String> = conn
                .query_row(
                    "SELECT permissions FROM plugin WHERE name=?1",
                    params![name],
                    |r| r.get(0),
                )
                .ok();
            let mut allowed = false;
            if let Some(pj) = perms_json {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&pj) {
                    if let Some(arr) = val
                        .get("net")
                        .and_then(|n| n.get("domains"))
                        .and_then(|d| d.as_array())
                    {
                        for d in arr {
                            if let Some(dom) = d.as_str() {
                                if host.eq_ignore_ascii_case(dom)
                                    || (dom.starts_with('.') && host.ends_with(dom))
                                {
                                    allowed = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !allowed {
                return Err("forbidden_net_domain".into());
            }
        }
    }
    // FS roots allowlist
    if method.starts_with("fs.") {
        let params_v = parsed.get("params").cloned().unwrap_or(serde_json::json!({}));
        let req_path = params_v.get("path").and_then(|v| v.as_str()).unwrap_or("");
        if !req_path.is_empty() {
            let conn = db.0.lock();
            let perms_json: Option<String> = conn
                .query_row(
                    "SELECT permissions FROM plugin WHERE name=?1",
                    params![name],
                    |r| r.get(0),
                )
                .ok();
            let mut allowed = false;
            if let Some(pj) = perms_json {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&pj) {
                    if let Some(arr) = val
                        .get("fs")
                        .and_then(|fs| fs.get("roots"))
                        .and_then(|r| r.as_array())
                    {
                        let target_canon: PathBuf = Path::new(req_path)
                            .canonicalize()
                            .unwrap_or_else(|_| PathBuf::from(req_path));
                        for root in arr {
                            if let Some(root_s) = root.as_str() {
                                let root_canon: PathBuf = Path::new(root_s)
                                    .canonicalize()
                                    .unwrap_or_else(|_| PathBuf::from(root_s));
                                if target_canon.starts_with(&root_canon) {
                                    allowed = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !allowed {
                return Err("forbidden_fs_root".into());
            }
        }
    }
    Ok(())
}
