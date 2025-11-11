# Providers — agent-editor

This guide explains the provider registry, key handling, defaults, and UI/CLI flows.

## Concepts
- Provider: `{name, kind: 'local'|'remote', enabled: 0/1, config: JSON}` in `provider` table.
- Keys: stored via OS keychain when built with `keyring` feature; DB fallback stores only a `key_set` flag (no secret material).
- Defaults (resolution order): repo.settings.default_provider → app_setting.default_provider → `local`.

## IPC / RPC
- List: `ai_providers_list()`
- Enable/disable: `ai_providers_enable|disable(name)`
- Keys: `ai_provider_key_set|get(name)`
- Model (example: openrouter): `ai_provider_model_get|set(name, model)`
- Resolve effective provider for doc: `ai_provider_resolve(doc_id, provider)` → `{name, kind, enabled, has_key, allowed}`
- AI run: `ai_run(provider|"default", doc_id, anchor_id?, prompt)`

## UI flows
- Settings → Providers: enable/disable, set keys, set global default, set provider model.
- Repo page: shows effective default per repo, with control to set it.
- Doc page: shows a provider chip and disables Run AI if provider not allowed; hints appear when disabled.

## CLI flows
- `agent-editor ai providers list|enable|disable|test`
- `agent-editor settings default-provider set <name>`
- `agent-editor repo set-default-provider <repo> <name>` (via repos_set_default_provider RPC)

## Redaction & Privacy
- `ai_run` builds minimal context and passes it through `redact()` to mask common secrets (AWS keys, bearer tokens, api_key params, high-entropy tokens).
- Remote providers require key and explicit enable. Provider metadata (provider/model) is stored in `ai_trace.response` for transparency.
