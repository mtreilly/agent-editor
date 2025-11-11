# agent-editor

Local-first Markdown knowledge system with backlinks, fast FTS, line-level AI context, deep repo scanning, and plugins. Desktop via Tauri 2; frontend via React 19 + Vite; CLI/TUI companion in Go.

- SQLite (FTS5) as the single source of truth
- .gitignore-aware scanner + file watcher
- Wiki-links and graph (neighbors/backlinks)
- Milkdown editor with custom schema for anchors and wiki-links
- JSON-RPC sidecar for CLI; Tauri IPC for UI

## Quick Start
```bash
pnpm install
pnpm dev         # launches Vite + Tauri dev (desktop)
```

Health check (dev)
```bash
pnpm dev:check   # calls /rpc repos_list on 127.0.0.1:35678
```

## Build
```bash
pnpm build
pnpm tauri build
```

See docs/guides/BUILD.md for prerequisites and troubleshooting.

## Learn the Codebase
- Code map: `docs/guides/CODEMAP.md` (where things live and why)
- RPC reference: `docs/manual/RPC.md`
- Data model: `docs/manual/DATA_MODEL.md`
- Providers guide: `docs/guides/PROVIDERS.md`
- Plugins guide: `docs/guides/PLUGINS.md`
- ElectricSQL prep: `docs/guides/ELECTRIC.md`
 - CLI quick ref: `docs/guides/CLI.md`
 - Development: `docs/guides/DEVELOPMENT.md`, CI: `docs/guides/CI.md`
 - Troubleshooting: `docs/guides/TROUBLESHOOTING.md`

## Tests
- E2E (web stubs): `HEADLESS=1 pnpm tmux:e2e`
- Smoke (sidecar + CLI): `pnpm tmux:smoke` or `pnpm smoke:cli`
- CI smoke: `HEADLESS=1 pnpm tmux:ci-smoke`

## Contributing
See `CONTRIBUTING.md` for conventions and safety guidelines.

## Dev Workflow (tmux + vibe)
- Use tmux scripts for dev/smoke/bench/tests: see `AGENTS.md` → Vibe + Tmux.
- Send start/progress/done notifications via `pnpm vibe:*` (optional channel via `VIBE_CHANNEL`).

## CLI
- Binary lives under `cli/`. Example commands:
```bash
# Scan + watch a repo
agent-editor repo add /abs/path && agent-editor repo scan /abs/path --watch

# Search
agent-editor doc search "your query" -o json

# Graph
agent-editor graph neighbors <doc-id> -o json
agent-editor graph backlinks <doc-id> -o json

# AI run
agent-editor ai run <doc-id> --provider local --prompt "Summarize"

# Bench
agent-editor fts bench --query the --n 50 -o json
```
