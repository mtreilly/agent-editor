# RPC — agent-editor

This is a living list of Tauri IPC/JSON-RPC methods and their intent. See `src-tauri/src/commands.rs` for signatures and return types.

## Repos
- `repos_add(path, name?, include?, exclude?)`
- `repos_list()`
- `repos_info(idOrName)`
- `repos_remove(idOrName)`
- `repos_set_default_provider(idOrName, provider)`

## Docs
- `docs_create(payload)` — `{ repo_id, slug, title, body }`
- `docs_update(payload)` — `{ doc_id, body, message? }`
- `docs_get(docId, content?)`
- `docs_delete(docId)`

## Search & Graph
- `search(repoId?, query, limit?, offset?)`
- `graph_neighbors(docId, depth?)`
- `graph_backlinks(docId)`
- `graph_related(docId)`
- `graph_path(startId, endId)`

## AI Providers
- `ai_run(provider, docId, anchorId?, prompt)`
- `ai_provider_key_set(name, key)` / `ai_provider_key_get(name)`
- `ai_provider_test(name, prompt?)`
- `ai_provider_model_get(name)` / `ai_provider_model_set(name, model)`
- `ai_provider_resolve(docId?, provider?)`

## Plugins
- `plugins_list()` / `plugins_info(name)` / `plugins_remove(name)`
- `plugins_enable(name)` / `plugins_disable(name)` / `plugins_upsert(name, ...)`
- `plugins_spawn_core(name, exec, args?)` / `plugins_shutdown_core(name)`
- `plugins_call_core(name, line)`
- `plugins_core_list()`

## Anchors
- `anchors_upsert(docId, anchorId, line)`
- `anchors_list(docId)`
- `anchors_delete(anchorId)`

## Sidecar
- `serve_api_start(port?)` — start JSON-RPC HTTP server (127.0.0.1:35678)
