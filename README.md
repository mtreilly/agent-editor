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
