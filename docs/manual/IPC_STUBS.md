# IPC Stubs — agent-editor

The React app imports `src/ipc/client.ts` which wraps Tauri `invoke` calls. In non-Tauri contexts (web tests/dev), a set of stubs return plausible values to allow UI to render and E2E tests to pass.

## Behavior
- `docs_get`: returns minimal doc metadata using the passed id as slug/title.
- `graph_*`: return empty arrays or a trivial path pair.
- `repos_*`: returns a demo repo when listing; updates no-op.
- `ai_providers_*`: returns a minimal provider list and defaults.
- `ai_provider_resolve`: returns `allowed=false` when docId includes `disabled` to simulate disabled providers in E2E.
- `ai_run`: returns `{ trace_id, text, provider, model }` without network.

This allows Playwright tests to validate UI wiring without requiring a desktop runtime.
