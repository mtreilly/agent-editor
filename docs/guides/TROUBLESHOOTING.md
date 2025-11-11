# Troubleshooting — agent-editor

## Common issues

- Port 5173 in use (Vite)
  - Symptom: Playwright or dev server logs: "Port 5173 is already in use".
  - Fix: Kill the stray Vite process or set `VITE_PORT` alternative; rely on `reuseExistingServer` in Playwright.

- JSON-RPC health check fails
  - Run `pnpm dev:check` (expects sidecar on 127.0.0.1:35678).
  - Start via `pnpm rpc:dev` or run desktop with `pnpm dev`.

- Tauri packaging fails due to icon
  - Ensure `src-tauri/icons/icon.png` is 32-bit RGBA (e.g., 512×512).

- Keyring not enabled (provider keys)
  - Builds without `keyring` store only a `key_set` flag in DB; no secret persisted. Enable feature for OS keychain storage.

- E2E flakiness due to server reuse
  - The web tests reuse existing Vite; ensure only one dev server is running or let Playwright reuse it.

- Scanner permissions / file access
  - Scanner obeys .gitignore; ensure files exist and are readable.

## Where to look
- Sidecar logs: `.sidecar.log` or your tmux pane running the sidecar.
- IPC errors: Browser devtools console in desktop.
- DB: Inspect `.dev/agent-editor.db*` for local dev.
