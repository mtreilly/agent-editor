# Data Model — agent-editor

Source of truth: `src-tauri/schema.sql`. Highlights below.

## Core tables
- repo(id, name, path, settings, created_at, updated_at)
- folder(id, repo_id, parent_id, path, slug, timestamps)
- doc(id, repo_id, folder_id, slug, title, lang, is_deleted, current_version_id, size_bytes, line_count, backlink_count, timestamps)
- doc_blob(id, content, encoding, mime, size_bytes)
- doc_version(id, doc_id, blob_id, author, message, created_at, hash)
- doc_asset(id, doc_id, filename, mime, size_bytes, blob_id, created_at) — attachments/binary assets linked to docs; filename unique per doc.
- link(id, repo_id, from_doc_id, to_doc_id?, to_slug, type, line_start, line_end, created_at)
- provenance(id, entity_type, entity_id, source, meta, created_at) — anchors stored here
- scan_job(id, repo_id, status, stats, started_at, finished_at, error)
- ai_trace(id, repo_id, doc_id, anchor_id, provider, request, response, input_tokens, output_tokens, cost_usd, created_at)
- plugin(id, name, version, kind, manifest, permissions, enabled, installed_at)
- plugin_event(id, plugin_id, type, payload, created_at)
- app_setting(key, value, updated_at)
- provider(name, kind, enabled, config, created_at, updated_at)

## FTS5
- `doc_fts` external-content virtual table (content_rowid = doc.rowid). Updates are managed in app code (delete+insert) for determinism.

## IDs & hashes
- IDs are UUIDv4 unless otherwise noted.
- `doc_version.hash` uses scoped hash `{doc_id}:{content_hash}` to avoid cross-doc collisions.

## Derived data
- `link` is derived from doc content on create/update/scan.
- `backlink_count` maintained from `link`.

## Invariants
- FTS doc count equals doc count for non-deleted docs.
- `doc.current_version_id` points to the latest version.
