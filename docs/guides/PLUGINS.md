# Plugins — agent-editor

This guide explains the UI/Core plugin model, permissions, and demos.

## Overview
- UI Plugins (TS): loaded dynamically in the web app; contribute commands/views/renderers.
- Core Plugins (process): spawned by the host (Rust) and communicate over JSON-RPC on stdin/stdout.

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

## Demos (tmux)
- RPC/FS: `pnpm tmux:plugin-rpc-demo`
- Net domains: `pnpm tmux:plugin-net-demo`
- DB gates: `pnpm tmux:plugin-db-demo`
- HEADLESS mode supported by setting `HEADLESS=1`.

## Sample plugin
- `plugins/echo-core/echo.js` — minimal Node echo over JSON-RPC, used by demos.

## Tests
- Unit tests cover capability gates and invalid envelopes in `commands.rs`.
