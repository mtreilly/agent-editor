use crate::secrets;
use crate::db::Db;

#[derive(serde::Serialize)]
struct OrMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(serde::Serialize)]
struct OrRequest<'a> {
    model: &'a str,
    messages: Vec<OrMessage<'a>>,
}

#[derive(serde::Deserialize)]
struct OrChoiceMsg {
    content: Option<String>,
}

#[derive(serde::Deserialize)]
struct OrChoice {
    message: Option<OrChoiceMsg>,
}

#[derive(serde::Deserialize)]
struct OrResponse {
    choices: Option<Vec<OrChoice>>,
}

// Minimal OpenRouter call using blocking reqwest with rustls.
// Reads model from provider.config->model if present; otherwise defaults to "openrouter/auto".
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct OrCallResult { pub text: String, pub model: String }

pub fn call_openrouter(db: &Db, prompt: &str, context: &str) -> Result<OrCallResult, String> {
    let key = secrets::provider_key_get(db, "openrouter")?;

    // Discover model config if set
    let model: String = {
        let conn = db.0.lock();
        conn.query_row(
            "SELECT COALESCE(json_extract(config,'$.model'),'openrouter/auto') FROM provider WHERE name='openrouter'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "openrouter/auto".to_string())
    };

    let user_content = format!("{}\n\n---\n{}", prompt, context);
    let req = OrRequest {
        model: &model,
        messages: vec![
            OrMessage { role: "system", content: "You are an AI assistant helping with code editing in an IDE. Be concise." },
            OrMessage { role: "user", content: &user_content },
        ],
    };

    let client = reqwest::blocking::Client::builder()
        .user_agent("agent-editor/0.0.0 (+https://example.local)")
        .build()
        .map_err(|e| format!("http_client_error: {}", e))?;

    let res: OrResponse = client
        .post("https://api.openrouter.ai/v1/chat/completions")
        .bearer_auth(key)
        .json(&req)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("http_error: {}", e))?
        .json()
        .map_err(|e| format!("decode_error: {}", e))?;

    if let Some(choices) = res.choices {
        for ch in choices {
            if let Some(msg) = ch.message {
                if let Some(text) = msg.content { return Ok(OrCallResult { text, model }); }
            }
        }
    }
    Err("empty_response".into())
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open_db, Db};
    use rusqlite::params;

    fn test_db() -> Db {
        let p = std::env::temp_dir().join(format!("ae-ai-test-{}.db", uuid::Uuid::new_v4()));
        open_db(&p).expect("open db")
    }

    #[test]
    fn provider_test_disabled() {
        let db = test_db();
        let conn = db.0.lock();
        conn.execute("UPDATE provider SET enabled=0 WHERE name='openrouter'", []).unwrap();
        drop(conn);
        let res = provider_test(&db, "openrouter", "ping");
        assert_eq!(res.unwrap_err(), "disabled");
    }

    #[test]
    fn provider_test_missing_key_remote() {
        let db = test_db();
        let conn = db.0.lock();
        conn.execute("UPDATE provider SET enabled=1 WHERE name='openrouter'", []).unwrap();
        drop(conn);
        let res = provider_test(&db, "openrouter", "ping");
        assert_eq!(res.unwrap_err(), "no_key");
    }
}
