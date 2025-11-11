# Configuration — agent-editor

## Precedence
props > env > config (DB/app settings) > defaults

## Environment variables
- `NODE_ENV` — development/production/test
- `LOG_LEVEL` — debug|info|warn|error
- `AGENT_API_ENDPOINT` — URL for agent API (if applicable)
- `AE_DB` — path to DB file for desktop; defaults to `src-tauri/.dev/agent-editor.db`
- `VIBE_CHANNEL` — Discord channel for `pnpm vibe:*` notifications

## App settings (DB table: app_setting)
- `default_provider` — global provider name when repo settings absent.

## Repo settings (repo.settings JSON)
- `default_provider` — default provider for the repo (overrides app).

## Provider config (provider.config JSON)
- Example: `{ "model": "openrouter/auto", "key_set": 1 }`
