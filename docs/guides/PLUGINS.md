# Plugins — agent-editor

This guide explains the UI/Core plugin model, permissions, and demos.

## Overview
- UI Plugins (TS): loaded dynamically in the web app; contribute commands/views/renderers.
- Core Plugins (process): spawned by the host (Rust) and communicate over JSON-RPC on stdin/stdout.

## Plugin Lifecycle

### Core Plugin Spawn

Core plugins run as separate processes and communicate via JSON-RPC 2.0 over stdin/stdout.

**Implementation:** `src-tauri/src/plugins/mod.rs`

#### spawn_core_plugin(spec: &CorePluginSpec)

Spawns a child process with:
- Executable and arguments from spec
- JSON-RPC channels on stdin/stdout
- stderr captured and logged with `[plugin:name:stderr]` prefix
- Process registered in global registry
- Double-spawn prevention

**Example:**
```rust
let spec = CorePluginSpec {
    name: "echo".to_string(),
    exec: "node".to_string(),
    args: vec!["plugins/echo-core/echo.cjs".to_string()],
    env: vec![],
    caps: Capabilities { ... },
};
spawn_core_plugin(&spec)?;
```

#### shutdown_core_plugin(name: &str)

Gracefully terminates a plugin:
- Unix: Sends SIGTERM, waits 5 seconds, sends SIGKILL if needed
- Windows: Immediate termination
- Cleans up from registry

#### call_core_plugin(name, method, params)

High-level API for calling plugin methods via JSON-RPC.

**JSON-RPC Format:**
```json
{"jsonrpc":"2.0","id":"uuid","method":"method_name","params":{...}}
```

#### Timeout Handling

- Default timeout: 30 seconds
- Configurable via `PLUGIN_CALL_TIMEOUT_MS` environment variable
- Timeout cancels the call and returns an error

#### Restart Policy

Plugins can automatically restart on crash with exponential backoff:
- Max 3 retries
- Delay: 200ms * 2^retry_count

### Testing

See `src-tauri/src/plugins/mod.rs` for unit tests covering:
- Spawn and shutdown lifecycle
- Double spawn prevention
- JSON-RPC communication
- Timeout handling

## Permissions model (Core)
- Envelope: JSON-RPC 2.0 `{jsonrpc,id,method,params}`.
- Host checks (subset):
  - `core.call` must be true and plugin `enabled=1` to accept calls.
  - `fs.write*` → `fs.write=true`; `fs.*` → `fs.read=true` and path under `fs.roots`.
  - `net.request*` → `net.request=true` and domain in `net.domains`.
  - `db.write*` → `db.write=true`; `db.*` → `db.query=true`.
  - `ai.invoke*` → `ai.invoke=true`.
  - `scanner.register*` → `scanner.register=true`.

## UI Host (TS)
- `src/plugins/host.ts` loads a plugin module and aggregates contributions.
- `app/routes/plugins.tsx` shows UI commands and a Core Plugins panel.

## Core Host (Rust)
- `plugins_spawn_core|plugins_shutdown_core|plugins_call_core|plugins_core_list` in `commands.rs`.
- Registry keeps child process handles; `plugins_core_list` returns `{name,pid,running}`.
 - Lifecycle polish:
   - Restart policy: auto-restart up to 3 times with exponential backoff when a core plugin exits unexpectedly.
   - Logging: core plugins' stderr is tailed into `.sidecar.log` with `[plugin:<name>]` prefixes; stdout lines returned from calls are also logged.
   - Call timeout: configurable via `PLUGIN_CALL_TIMEOUT_MS` (default 5000ms). On timeout, call fails with `timeout`; next call may trigger restart if the child exited.

## Demos (tmux)
- RPC/FS: `pnpm tmux:plugin-rpc-demo`
- Net domains: `pnpm tmux:plugin-net-demo`
- DB gates: `pnpm tmux:plugin-db-demo`
- HEADLESS mode supported by setting `HEADLESS=1`.

## Sample plugin
- `plugins/echo-core/echo.cjs` — minimal Node echo over JSON-RPC, used by demos.

### UI Plugin example (TypeScript)
```ts
// plugins/hello-world/index.ts
import type { PluginV1 } from '../../src/plugins/types'

const plugin: PluginV1 = {
  name: 'hello-world',
  version: '1.0.0',
  kind: 'ui',
  async activate(ctx) {
    return {
      commands: [
        { id: 'hello.say', title: 'Say Hello', run: async () => alert('Hello!') },
      ],
    }
  },
}
export default plugin
```

## Tests
- Unit tests cover capability gates and invalid envelopes in `commands.rs`.
