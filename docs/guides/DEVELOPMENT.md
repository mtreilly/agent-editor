# Development — agent-editor

## Daily workflow
- Bootstrap: `pnpm tmux:bootstrap` (install, cargo check, CLI build)
- Dev desktop: `pnpm tmux:dev` (Vite + Tauri)
- E2E (web stubs): `HEADLESS=1 pnpm tmux:e2e`
- Smoke (sidecar + CLI): `pnpm tmux:smoke` or `pnpm smoke:cli`
- Bench: `pnpm tmux:bench`
- Packaging: `pnpm tmux:tauri-build`

## Code search (fast)
- Use `fd`, `ag`, `ast-grep` per AGENTS.md; avoid find/grep/ls -R for repo-wide searches.

## Conventions
- Feature folders in `app/features` and Start routes in `app/routes`.
- Tauri commands live in `src-tauri/src/commands.rs`; keep signatures small and stable.
- IPC/Web stubs in `src/ipc/client.ts` allow E2E without Tauri.
- No barrel files; direct imports only.
- i18n: All visible strings must be in `public/locales/*/*.json`.

## Tests
- Rust unit tests live near code (e.g., in `commands.rs` modules).
- E2E tests in `tests/e2e/*.spec.ts` run against web stubs.
- Add focused unit tests for permission/redaction-like logic; keep E2E for user flows.

## Debugging
- `pnpm dev:check` for JSON-RPC health.
- Use tmux panes for logs (e.g., `.sidecar.log`, CLI outputs).
- Add `println!/console.log` sparingly; remove noisy logs before commit.

## Style & commits
- Conventional commits (`feat:`, `fix:`, `docs:`, ...). Scope by feature (`feat(editor): ...`).
- Prefer small, atomic PRs/commits with clear rationale.

## Safety
- Do not persist secrets. `secrets.rs` uses OS keychain when enabled; DB fallback only records presence.
- `redact()` masks common tokens before storing AI traces.
