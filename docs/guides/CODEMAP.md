# Code Map — agent-editor

This document orients you to the codebase so you can find the right place quickly and make safe changes.

## High-level
- Desktop app (Tauri 2, Rust) with a JSON-RPC sidecar for headless/CLI.
- Web UI (React 19 + TanStack Start) talks to Tauri IPC; web stubs backfill for tests.
- CLI (Go) calls the JSON-RPC sidecar.
- SQLite (FTS5) is the system of record. Derived state (FTS, links) is rebuilt deterministically.

## Rust (src-tauri/src)
- `db.rs` — open/seed DB; PRAGMAs; include `schema.sql`.
- `commands.rs` — Tauri commands:
  - repos_*: add/list/info/remove/set_default_provider
  - docs_*: create/update/get/delete
  - search, graph_*: neighbors/backlinks/related/path
  - ai_run, ai_provider_*: key set/get/test, model get/set, resolve
  - plugins_*: list/info/enable/disable/remove/upsert; spawn/shutdown/call; core_list
  - anchors_*: upsert/list/delete
  - Helper: redact(); tests for provider gating and plugin permissions
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
