//! AI provider and execution commands

use crate::{ai, db::Db, secrets};
use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Deserialize, Clone)]
pub struct AiRunRequest {
    pub provider: String,
    pub doc_id: String,
    pub anchor_id: Option<String>,
    pub line: Option<usize>,
    pub prompt: String,
}

#[derive(Serialize)]
pub struct ProviderRow {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
}

// ===== AI Run Commands =====

#[tauri::command]
pub async fn ai_provider_resolve(
    doc_id: Option<String>,
    provider: Option<String>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    // Resolve provider similar to ai_run_core
    let provider_name: String = {
        let conn = db.0.lock();
        let use_default = provider.as_deref().unwrap_or("default").is_empty()
            || provider.as_deref() == Some("default");
        if use_default {
            // Try repo default via doc_id if provided
            if let Some(ref did) = doc_id {
                let repo_id: Option<String> = conn
                    .query_row(
                        "SELECT d.repo_id FROM doc d WHERE d.id=?1 OR d.slug=?1",
                        params![did],
                        |r| r.get(0),
                    )
                    .ok();
                if let Some(rid) = repo_id {
                    let repo_default: Option<String> = conn
                        .query_row(
                            "SELECT json_extract(settings,'$.default_provider') FROM repo WHERE id=?1",
                            params![rid],
                            |r| r.get(0),
                        )
                        .ok();
                    if let Some(p) = repo_default.filter(|s: &String| !s.is_empty()) {
                        p
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
        conn.query_row(
            "SELECT kind, enabled FROM provider WHERE name=?1",
            params![&provider_name],
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
pub async fn ai_run(
    provider: String,
    doc_id: String,
    anchor_id: Option<String>,
    prompt: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let req = AiRunRequest {
        provider,
        doc_id,
        anchor_id,
        line: None,
        prompt,
    };
    ai_run_core(&db, req)
}

pub fn ai_run_core(
    db: &std::sync::Arc<Db>,
    req: AiRunRequest,
) -> Result<serde_json::Value, String> {
    // Resolve provider: if empty or "default", use repo.settings.default_provider; else use provided
    let (body, provider_name): (String, String) = {
        let conn = db.0.lock();
        // fetch body and repo_id
        let (body, repo_id): (String, String) = conn
            .query_row(
                "SELECT df.body, d.repo_id FROM doc_fts df JOIN doc d ON d.rowid=df.rowid WHERE d.id=?1 OR d.slug=?1",
                params![req.doc_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let use_default = req.provider.is_empty() || req.provider == "default";
        let provider = if use_default {
            // repo default, else global app default, else 'local'
            let repo_default: Option<String> = conn
                .query_row(
                    "SELECT json_extract(settings,'$.default_provider') FROM repo WHERE id=?1",
                    params![repo_id],
                    |r| r.get(0),
                )
                .ok();
            if let Some(p) = repo_default.filter(|s: &String| !s.is_empty()) {
                p
            } else {
                conn.query_row(
                    "SELECT value FROM app_setting WHERE key='default_provider'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "local".into())
            }
        } else {
            req.provider.clone()
        };
        (body, provider)
    };

    // Determine target line
    let mut line = req.line.unwrap_or(1);
    if let Some(aid) = &req.anchor_id {
        if let Some(parsed) = parse_anchor_line(aid) {
            line = parsed;
        }
    }

    let context = extract_context(&body, line, 12);
    let redacted = redact(&context);

    // Provider gating and simulated response (echo)
    {
        let conn = db.0.lock();
        if let Ok((kind, enabled)) = conn.query_row(
            "SELECT kind, enabled FROM provider WHERE name=?1",
            params![&provider_name],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        ) {
            if enabled == 0 {
                return Err("provider_disabled".into());
            }
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
            Ok(res) => {
                response_model = res.model.clone();
                res.text
            }
            Err(err) => format!(
                "[openrouter:error:{}]\nPrompt: {}\n---\n{}",
                err, req.prompt, redacted
            ),
        }
    } else {
        format!(
            "[{}]\nPrompt: {}\n---\n{}",
            provider_name, req.prompt, redacted
        )
    };

    // Persist ai_trace
    let conn = db.0.lock();
    let trace_id = Uuid::new_v4().to_string();
    let request_json = serde_json::json!({"prompt": req.prompt, "context": redacted});
    let response_json = serde_json::json!({"text": response_text, "provider": provider_name, "model": response_model});
    conn.execute(
        "INSERT INTO ai_trace(id,repo_id,doc_id,anchor_id,provider,request,response,input_tokens,output_tokens,cost_usd) VALUES(?, (SELECT repo_id FROM doc WHERE id=?2 OR slug=?2), ?2, ?, ?, ?, ?, 0, 0, 0.0)",
        params![
            trace_id,
            req.doc_id,
            req.anchor_id.unwrap_or_default(),
            provider_name,
            request_json.to_string(),
            response_json.to_string()
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "trace_id": trace_id,
        "text": response_text,
        "provider": provider_name,
        "model": response_model
    }))
}

fn parse_anchor_line(anchor_id: &str) -> Option<usize> {
    // Expected formats: anc_<doc>_<line> or anc_<doc>_<line>_<ver>
    let parts: Vec<&str> = anchor_id.split('_').collect();
    if parts.len() >= 3 {
        parts[parts.len() - 2]
            .parse::<usize>()
            .ok()
            .or_else(|| parts.last()?.parse::<usize>().ok())
    } else {
        None
    }
}

// ===== Provider Management =====

#[tauri::command]
pub async fn ai_providers_list(
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<Vec<ProviderRow>, String> {
    let conn = db.0.lock();
    let mut stmt = conn
        .prepare("SELECT name, kind, enabled FROM provider ORDER BY name ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ProviderRow {
                name: r.get(0)?,
                kind: r.get(1)?,
                enabled: r.get::<_, i64>(2)? != 0,
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
pub async fn ai_providers_enable(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "UPDATE provider SET enabled=1, updated_at=datetime('now') WHERE name=?1",
            params![name],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

#[tauri::command]
pub async fn ai_providers_disable(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let conn = db.0.lock();
    let n = conn
        .execute(
            "UPDATE provider SET enabled=0, updated_at=datetime('now') WHERE name=?1",
            params![name],
        )
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"updated": n>0}))
}

// ===== Provider Key Management =====

#[tauri::command]
pub async fn ai_provider_key_set(
    name: String,
    key: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let ok = secrets::provider_key_set(&db, &name, &key)?;
    Ok(serde_json::json!({"updated": ok}))
}

#[tauri::command]
pub async fn ai_provider_key_get(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let has = secrets::provider_key_exists(&db, &name)?;
    Ok(serde_json::json!({"has_key": has}))
}

// ===== Provider Model Configuration =====

#[tauri::command]
pub async fn ai_provider_model_get(
    name: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
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
pub async fn ai_provider_model_set(
    name: String,
    model: String,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
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
pub async fn ai_provider_test(
    name: String,
    prompt: Option<String>,
    db: State<'_, std::sync::Arc<Db>>,
) -> Result<serde_json::Value, String> {
    let res = ai::provider_test(&db, &name, &prompt.unwrap_or_else(|| "ping".into()))?;
    Ok(res)
}

// ===== Helper Functions =====

fn extract_context(body: &str, line: usize, n: usize) -> String {
    let lines: Vec<&str> = body.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let idx = if line == 0 { 0 } else { line - 1 };
    let start = idx.saturating_sub(n);
    let end = (idx + n + 1).min(lines.len());
    lines[start..end].join("\n")
}

fn redact(s: &str) -> String {
    let mut out = s.to_string();
    // AWS Access Key IDs (AKIA/ASIAxxxxxxxxxxxxxxxx)
    let re_aws_ak = Regex::new(r"(?i)\b(AKIA|ASIA)[0-9A-Z]{16}\b").unwrap();
    out = re_aws_ak.replace_all(&out, "****").to_string();

    // AWS Secret Access Key (40 chars base64-like)
    let re_aws_sk = Regex::new(
        r"(?i)(aws[_-]?secret[_-]?access[_-]?key\s*[:=]\s*['\x22]?)([A-Za-z0-9/+=]{40})",
    )
    .unwrap();
    out = re_aws_sk.replace_all(&out, "$1****").to_string();

    // Bearer tokens
    let re_bearer = Regex::new(r"(?i)\b(bearer)\s+[A-Za-z0-9_.]{16,}\b").unwrap();
    out = re_bearer.replace_all(&out, "$1 ****").to_string();

    // Generic api key/token param
    let re_key_param = Regex::new(
        r"(?i)(api[_-]?key|apikey|token|auth)_?id?\s*[:=]\s*['\x22]?([A-Za-z0-9_-]{16,})",
    )
    .unwrap();
    out = re_key_param.replace_all(&out, "$1=****").to_string();

    // URL query params ?key=..., &token=
    let re_query = Regex::new(r"([?&](?:key|api[_-]?key|token)=[^&\s]{4,})").unwrap();
    out = re_query
        .replace_all(&out, |caps: &regex::Captures| {
            let s = &caps[1];
            let k = s.split('=').next().unwrap_or("key");
            format!("{}=****", k)
        })
        .to_string();

    // High-entropy generic tokens: long hex/base64ish words (fallback)
    let re_entropy = Regex::new(r"\b[A-Za-z0-9/_\+=]{24,}\b").unwrap();
    out = re_entropy
        .replace_all(&out, |m: &regex::Captures| {
            let t = &m[0];
            // Avoid redacting typical prose by requiring mixed char classes
            let has_alpha = t.chars().any(|c| c.is_ascii_alphabetic());
            let has_digit = t.chars().any(|c| c.is_ascii_digit());
            if has_alpha && has_digit {
                "****".to_string()
            } else {
                t.to_string()
            }
        })
        .to_string();

    out
}
