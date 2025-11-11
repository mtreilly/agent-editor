# Scripts — agent-editor

## tmux scripts
- `pnpm tmux:bootstrap` — install, cargo check, CLI build
- `pnpm tmux:dev` — start Vite + Tauri desktop
- `pnpm tmux:smoke` — sidecar + CLI smoke
- `pnpm tmux:e2e` — web E2E (Playwright)
- `pnpm tmux:bench` — FTS and scan benches
- `pnpm tmux:provider-demo` — provider demo
- `pnpm tmux:plugin-*-demo` — core plugin demos (rpc/net/db)
- `pnpm tmux:plugin-log-smoke` — tails plugin-prefixed lines via CLI (`plugin events tail`), starts echo core, simulates crash to exercise auto-restart, and issues a follow-up call
- `pnpm tmux:tauri-build` — package desktop
- `pnpm tmux:ci-smoke` — smoke + e2e in headless panes

## vibe notifications
- `pnpm vibe:start|progress|done` — send Discord messages (set `VIBE_CHANNEL` if needed).
- CI/headless fallback: `vibe discord message send --content "..."` (uses default channel from config)
