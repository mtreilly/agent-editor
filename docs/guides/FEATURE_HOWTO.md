# How to Add a Feature — agent-editor

This checklist walks you through adding a user-facing feature end‑to‑end.

## 1) UI route and component
- Create `app/routes/<feature>.tsx` with `createFileRoute()` and a small component.
- Extract visible strings to `public/locales/en/<feature>.json`.
- Add keyboard and ARIA per `docs/guides/A11Y.md`.
- Add a Playwright E2E test under `tests/e2e/<feature>.spec.ts`.

## 2) IPC and RPC (if needed)
- Add a wrapper in `src/ipc/client.ts` for new Tauri command(s). Provide a sane web stub for tests.
- Add a `#[tauri::command]` in `src-tauri/src/commands.rs`; register it in `src-tauri/src/main.rs`.
- Release DB locks quickly; return structured values.
- If CLI access is required, add a cobra subcommand in `cli/cmd/*.go` calling the new RPC via `cli/internal/rpc`.

## 3) Scanner/DB/Graph changes (if needed)
- Update schema and writes in a single transaction when touching doc/link/fts.
- Keep FTS updates deterministic (delete+insert strategy).

## 4) Tests
- Rust unit tests for permissions/validation/redaction‑like logic.
- E2E tests for UI flows (web stubs acceptable unless desktop integration is required).

## 5) Docs
- Update `docs/manual/RPC.md` and `docs/guides/CODEMAP.md` if APIs/locations changed.
- Add a short note in `docs/progress/STATUS.md` or `docs/OPEN_QUESTIONS.md` for deferred items.

## 6) Quality checks
- i18n extraction; keyboard navigation; focus handling; WCAG AA contrast.
- Conventional commits; small atomic changes; use `fd/ag/ast-grep` for search.

See also: `docs/guides/ROUTING.md`, `docs/guides/SCANNER.md`, `docs/guides/TESTING.md`, `docs/manual/RPC.md`.
