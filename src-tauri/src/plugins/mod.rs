//! Plugin Host scaffolding (v0)
//! Core plugin processes run as children with capability grants; UI plugins load in frontend.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use serde_json::Value;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Capabilities {
    pub fs_roots: Vec<String>,      // allowed read roots; write implied if db_write is true and path is under roots
    pub net_domains: Vec<String>,   // allowed domains for net access
    pub db_read: bool,              // allow read-only DB queries (through host RPC, not direct)
    pub db_write: bool,             // allow DB mutations (guarded by host)
    pub ai_providers: Vec<String>,  // allowed AI providers
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CorePluginSpec {
    pub name: String,
    pub exec: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub caps: Capabilities,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct CorePluginHandle {
    pub name: String,
}

struct PluginProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    exec: String,
    args: Vec<String>,
    restart_count: u32,
    max_restarts: u32,
    backoff_ms: u64,
}

static PLUGIN_REGISTRY: OnceLock<Mutex<HashMap<String, PluginProcess>>> = OnceLock::new();

fn get_registry() -> &'static Mutex<HashMap<String, PluginProcess>> {
    PLUGIN_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn log_plugin_line(name: &str, stream: &str, line: &str) {
    eprintln!("[plugin:{}:{}] {}", name, stream, line.trim_end());
}

fn spawn_child_process(name: &str, exec: &str, args: &[String], env: &[(String, String)]) -> Result<(Child, Option<ChildStdin>, Option<ChildStdout>), String> {
    let mut cmd = Command::new(exec);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn_failed: {}", e))?;

    // Spawn stderr logger thread
    if let Some(stderr) = child.stderr.take() {
        let name_owned = name.to_string();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => log_plugin_line(&name_owned, "stderr", &line),
                    Err(_) => break,
                }
            }
        });
    }

    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    Ok((child, stdin, stdout))
}

/// Spawn a core plugin process with JSON-RPC 2.0 channels on stdin/stdout
pub fn spawn_core_plugin(spec: &CorePluginSpec) -> Result<CorePluginHandle, String> {
    let registry = get_registry();
    let mut map = registry.lock().map_err(|_| "lock_poison".to_string())?;

    if map.contains_key(&spec.name) {
        return Err("already_running".to_string());
    }

    let (child, stdin, stdout) = spawn_child_process(&spec.name, &spec.exec, &spec.args, &spec.env)?;

    map.insert(spec.name.clone(), PluginProcess {
        child,
        stdin,
        stdout,
        exec: spec.exec.clone(),
        args: spec.args.clone(),
        restart_count: 0,
        max_restarts: 3,
        backoff_ms: 200,
    });

    Ok(CorePluginHandle {
        name: spec.name.clone(),
    })
}

/// Shutdown a core plugin with graceful termination (SIGTERM then SIGKILL)
pub fn shutdown_core_plugin(name: &str) -> Result<(), String> {
    let registry = get_registry();
    let mut map = registry.lock().map_err(|_| "lock_poison".to_string())?;

    if let Some(mut proc) = map.remove(name) {
        // Try graceful shutdown first
        #[cfg(unix)]
        {
            // Send SIGTERM
            unsafe {
                libc::kill(proc.child.id() as i32, libc::SIGTERM);
            }

            // Wait up to 5 seconds for graceful shutdown
            let start = std::time::Instant::now();
            let timeout = Duration::from_secs(5);

            while start.elapsed() < timeout {
                match proc.child.try_wait() {
                    Ok(Some(_)) => return Ok(()), // Process exited
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(e) => return Err(format!("wait_failed: {}", e)),
                }
            }

            // If still alive, send SIGKILL
            let _ = proc.child.kill();
        }

        #[cfg(not(unix))]
        {
            // On Windows, just kill immediately
            let _ = proc.child.kill();
        }

        let _ = proc.child.wait();
        Ok(())
    } else {
        Err("not_found".to_string())
    }
}

/// Call a core plugin with a raw JSON-RPC 2.0 line
/// This is used by the commands layer which receives pre-formed JSON-RPC requests
pub fn call_core_plugin_raw(name: &str, line: &str) -> Result<Value, String> {
    call_core_plugin_raw_with_timeout(name, line, Duration::from_secs(30))
}

/// Call a core plugin with JSON-RPC 2.0 request/response
/// Returns the JSON-RPC response or an error
pub fn call_core_plugin(name: &str, method: &str, params: Value) -> Result<Value, String> {
    call_core_plugin_with_timeout(name, method, params, Duration::from_secs(30))
}

/// Call a core plugin with a custom timeout
pub fn call_core_plugin_with_timeout(
    name: &str,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, String> {
    // Build JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let request_line = serde_json::to_string(&request).map_err(|e| format!("json_error: {}", e))?;
    call_core_plugin_raw_with_timeout(name, &request_line, timeout)
}

/// Call a core plugin with a raw JSON-RPC line and custom timeout
pub fn call_core_plugin_raw_with_timeout(
    name: &str,
    line: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let registry = get_registry();
    let mut map = registry.lock().map_err(|_| "lock_poison".to_string())?;

    let proc = map.get_mut(name).ok_or_else(|| "not_found".to_string())?;

    // Check if process has exited and attempt restart if needed
    if proc.child.try_wait().ok().flatten().is_some() {
        if proc.restart_count < proc.max_restarts {
            let backoff = Duration::from_millis(proc.backoff_ms * (1 << proc.restart_count));
            drop(map); // Release lock during sleep
            std::thread::sleep(backoff);
            map = registry.lock().map_err(|_| "lock_poison".to_string())?;

            let proc = map.get_mut(name).ok_or_else(|| "not_found".to_string())?;
            let (new_child, new_stdin, new_stdout) = spawn_child_process(name, &proc.exec, &proc.args, &[])?;
            proc.child = new_child;
            proc.stdin = new_stdin;
            proc.stdout = new_stdout;
            proc.restart_count += 1;
        } else {
            return Err("max_restarts_exceeded".to_string());
        }
    }

    let proc = map.get_mut(name).ok_or_else(|| "not_found".to_string())?;

    // Write request to stdin
    let stdin = proc.stdin.as_mut().ok_or_else(|| "stdin_closed".to_string())?;
    stdin.write_all(line.as_bytes()).map_err(|e| format!("write_error: {}", e))?;
    stdin.write_all(b"\n").map_err(|e| format!("write_error: {}", e))?;
    stdin.flush().map_err(|e| format!("flush_error: {}", e))?;

    // Read response from stdout with timeout
    let stdout = proc.stdout.take().ok_or_else(|| "stdout_closed".to_string())?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buf = String::new();
        let res = reader.read_line(&mut buf).map_err(|e| e.to_string());
        let stdout_back = reader.into_inner();
        let _ = tx.send((stdout_back, res, buf));
    });

    match rx.recv_timeout(timeout) {
        Ok((stdout_back, res, buf)) => {
            // Return stdout to proc
            if let Some(proc) = map.get_mut(name) {
                proc.stdout = Some(stdout_back);
            }

            res?;
            let trimmed = buf.trim();
            if !trimmed.is_empty() {
                log_plugin_line(name, "stdout", trimmed);
            }

            if trimmed.is_empty() {
                return Ok(serde_json::json!({"ok": true}));
            }

            let response: Value = serde_json::from_str(trimmed)
                .map_err(|e| format!("json_parse_error: {}", e))?;

            // Check for JSON-RPC error
            if let Some(error) = response.get("error") {
                return Err(format!("rpc_error: {}", error));
            }

            Ok(response)
        }
        Err(_) => {
            // On timeout, stdout is lost (still in the thread)
            // Next call will trigger restart policy
            Err("timeout".to_string())
        }
    }
}

/// List all running core plugins
pub fn list_core_plugins() -> Vec<(String, u32, bool)> {
    let registry = get_registry();
    if let Ok(mut map) = registry.lock() {
        map.iter_mut()
            .map(|(name, proc)| {
                let running = proc.child.try_wait().ok().flatten().is_none();
                let pid = proc.child.id();
                (name.clone(), pid, running)
            })
            .collect()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    fn get_echo_plugin_path() -> PathBuf {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        PathBuf::from(manifest_dir).join("../plugins/echo-core/echo.cjs")
    }

    #[test]
    fn test_spawn_and_shutdown() {
        let echo_path = get_echo_plugin_path();
        if !echo_path.exists() {
            eprintln!("Skipping test: echo plugin not found at {:?}", echo_path);
            return;
        }

        let spec = CorePluginSpec {
            name: "test-echo".to_string(),
            exec: "node".to_string(),
            args: vec![echo_path.to_string_lossy().to_string()],
            env: vec![],
            caps: Capabilities {
                fs_roots: vec![],
                net_domains: vec![],
                db_read: false,
                db_write: false,
                ai_providers: vec![],
            },
        };

        let handle = spawn_core_plugin(&spec).expect("spawn failed");
        assert_eq!(handle.name, "test-echo");

        // Verify it's in the list
        let list = list_core_plugins();
        assert!(list.iter().any(|(name, _, running)| name == "test-echo" && *running));

        // Shutdown
        shutdown_core_plugin("test-echo").expect("shutdown failed");

        // Verify shutdown error on double shutdown
        let result = shutdown_core_plugin("test-echo");
        assert!(result.is_err());
    }

    #[test]
    fn test_double_spawn_prevention() {
        let echo_path = get_echo_plugin_path();
        if !echo_path.exists() {
            eprintln!("Skipping test: echo plugin not found");
            return;
        }

        let spec = CorePluginSpec {
            name: "test-double".to_string(),
            exec: "node".to_string(),
            args: vec![echo_path.to_string_lossy().to_string()],
            env: vec![],
            caps: Capabilities {
                fs_roots: vec![],
                net_domains: vec![],
                db_read: false,
                db_write: false,
                ai_providers: vec![],
            },
        };

        spawn_core_plugin(&spec).expect("first spawn failed");
        let result = spawn_core_plugin(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already_running"));

        shutdown_core_plugin("test-double").ok();
    }

    #[test]
    fn test_call_plugin() {
        let echo_path = get_echo_plugin_path();
        if !echo_path.exists() {
            eprintln!("Skipping test: echo plugin not found");
            return;
        }

        let spec = CorePluginSpec {
            name: "test-call".to_string(),
            exec: "node".to_string(),
            args: vec![echo_path.to_string_lossy().to_string()],
            env: vec![],
            caps: Capabilities {
                fs_roots: vec![],
                net_domains: vec![],
                db_read: false,
                db_write: false,
                ai_providers: vec![],
            },
        };

        spawn_core_plugin(&spec).expect("spawn failed");

        let params = serde_json::json!({"test": "value"});
        let response = call_core_plugin("test-call", "test.method", params)
            .expect("call failed");

        assert!(response.get("result").is_some());

        shutdown_core_plugin("test-call").ok();
    }

    #[test]
    fn test_timeout_handling() {
        let echo_path = get_echo_plugin_path();
        if !echo_path.exists() {
            eprintln!("Skipping test: echo plugin not found");
            return;
        }

        let spec = CorePluginSpec {
            name: "test-timeout".to_string(),
            exec: "node".to_string(),
            args: vec![echo_path.to_string_lossy().to_string()],
            env: vec![],
            caps: Capabilities {
                fs_roots: vec![],
                net_domains: vec![],
                db_read: false,
                db_write: false,
                ai_providers: vec![],
            },
        };

        spawn_core_plugin(&spec).expect("spawn failed");

        // Call with very short timeout - might timeout depending on system
        let params = serde_json::json!({});
        let result = call_core_plugin_with_timeout(
            "test-timeout",
            "test.method",
            params,
            Duration::from_millis(1),
        );

        // Either succeeds quickly or times out
        if let Err(e) = result {
            assert!(e.contains("timeout"));
        }

        shutdown_core_plugin("test-timeout").ok();
    }
}

