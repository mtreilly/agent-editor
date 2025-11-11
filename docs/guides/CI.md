# CI — agent-editor

This checklist helps wire CI to run fast smokes and e2e in headless environments.

## Scripts
- `pnpm tmux:bootstrap` — install + cargo check + CLI build
- `HEADLESS=1 pnpm tmux:smoke` — sidecar + CLI smoke (asserts scan/search/fts invariants)
- `HEADLESS=1 pnpm tmux:e2e` — web-only E2E with Playwright stubs
- `HEADLESS=1 pnpm tmux:tauri-build` — desktop packaging build
- `pnpm ci:bench` — runs both benches and asserts thresholds:
  - FTS latency: p95/p99/avg (override with `FTS_P95_MS`, `FTS_P99_MS`, `FTS_AVG_MS`)
  - Scan throughput: docs/sec on synthetic dataset (override with `SCAN_DOCS_PER_SEC_MIN`, default 1000)

## Suggested CI job order
1) Node install cache + `pnpm install`
2) `pnpm dev:check` (quick RPC check if sidecar is up from a prior step)
3) `HEADLESS=1 pnpm tmux:smoke`
4) `HEADLESS=1 pnpm tmux:e2e`
5) (optional) `HEADLESS=1 pnpm tmux:tauri-build`

## Notes
- E2E uses web IPC stubs; to exercise real flows, point tests at the desktop app or a local RPC proxy.
- Ensure `VIBE_CHANNEL` is configured if you want Discord notifications from `pnpm vibe:*` scripts.
