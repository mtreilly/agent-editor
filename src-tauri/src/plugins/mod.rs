//! Plugin Host scaffolding (v0)
//! Core plugin processes run as children with capability grants; UI plugins load in frontend.

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
pub struct CorePluginHandle {
    pub name: String,
    // In future: child process handle, IPC channels, etc.
}

#[allow(dead_code)]
pub fn spawn_core_plugin(_spec: &CorePluginSpec) -> Result<CorePluginHandle, String> {
    // Placeholder: actual spawn/IPC wiring to be implemented in M3
    Err("not implemented: core plugin spawn".into())
}

#[allow(dead_code)]
pub fn shutdown_core_plugin(_name: &str) -> Result<(), String> {
    // Placeholder
    Ok(())
}

