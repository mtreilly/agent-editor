# Testing — agent-editor

## Strategy
- Unit tests: Rust (permissions, redaction, parsers) and TS where practical.
- E2E tests: Playwright against web IPC stubs; focus on user flows and a11y.
- Benches: CLI benches for FTS latency and scan throughput (tmux benches + scripts).

## Running
- Unit (Rust): `cd src-tauri && cargo test`
- E2E (web stubs): `HEADLESS=1 pnpm tmux:e2e`
- Smoke (sidecar + CLI): `pnpm tmux:smoke`
- CI smoke (combined): `HEADLESS=1 pnpm tmux:ci-smoke`

## Adding tests
- Rust unit tests can live in the same file (e.g., `commands.rs` modules) under `#[cfg(test)]`.
- For E2E, add `tests/e2e/<feature>.spec.ts`; use role-based selectors; ensure stubs return plausible results.
- Prefer small, focused tests; keep cross-cutting assertions in smoke.
