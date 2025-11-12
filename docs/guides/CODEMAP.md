# Code Map — agent-editor

This document orients you to the codebase so you can find the right place quickly and make safe changes.

## High-level
- Desktop app (Tauri 2, Rust) with a JSON-RPC sidecar for headless/CLI.
- Web UI (React 19 + TanStack Start) talks to Tauri IPC; web stubs backfill for tests.
- CLI (Go) calls the JSON-RPC sidecar.
- SQLite (FTS5) is the system of record. Derived state (FTS, links) is rebuilt deterministically.

## Rust (src-tauri/src)
- `db.rs` — open/seed DB; PRAGMAs; include `schema.sql`.
- `commands/` — Tauri IPC command implementations, organized by feature
  - `mod.rs` - Re-exports all command modules
  - `repo.rs` (126 lines) - Repository management: add, list, info, remove, set_default_provider
  - `settings.rs` (42 lines) - App settings: get, set
  - `scan.rs` (106 lines) - Repository scanning with filters
  - `doc.rs` (234 lines) - Document CRUD operations
  - `search.rs` (57 lines) - Full-text search via FTS5
  - `graph.rs` (149 lines) - Graph queries: backlinks, neighbors, related, path
  - `anchor.rs` (73 lines) - Document anchor management
  - `export.rs` (1063 lines) - Import/export docs and database
  - `ai.rs` (436 lines) - AI provider management and execution
  - `plugin.rs` (370 lines) - Plugin lifecycle and management
  - **Pattern:** Each module contains related Tauri commands marked with `#[tauri::command]`; re-exported through `mod.rs` for use in `main.rs`; helper functions and types kept within their respective modules; all modules under 500 lines (except export.rs which handles complex archive logic)
- `api.rs` — JSON-RPC endpoint; maps to the same core logic.
- `scan/` — .gitignore-aware scanner and watcher; upserts; dedupe; FTS maintenance.
- `graph/` — link extraction + graph queries and tests.
- `ai/` — provider adapters (e.g., OpenRouter); shared types.
- `secrets.rs` — keychain facade (`keyring` feature) with DB fallback flag.

## Web UI (app/)
- `routes/` — Start-style route files: `index`, `search`, `repo`, `doc.$id`, `graph.$id`, `settings.providers`, `plugins`.
- `features/`
  - `editor/` — Milkdown editor + anchors panel
  - `commands/` — command palette and registry
- `src/ipc/client.ts` — Unified IPC with web stubs used by Playwright.

## CLI (cli/)
- cobra structure: `repo`, `doc`, `graph`, `fts`, `ai`, `plugin`, `serve`, `settings`.
- JSON-RPC client: `internal/rpc/client.go`.

## Docs to know
- Plan: `docs/plans/MASTER_PLAN.md`
- Build: `docs/guides/BUILD.md`, Smoke: `docs/guides/SMOKE.md`
- Providers: `docs/guides/PROVIDERS.md`
- Plugins: `docs/guides/PLUGINS.md`
- RPC/API: `docs/manual/RPC.md`
- Data model: `docs/manual/DATA_MODEL.md`
- ElectricSQL prep: `docs/guides/ELECTRIC.md`

## Change safety
- Add tests next to code for Rust and TS where possible.
- Prefer small, isolated changes; update guides when touching public APIs.
- Keep IPC contracts stable; if changing, update `RPC.md` and CLI map.
