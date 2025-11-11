# ElectricSQL Prep — agent-editor

This guide captures the proposed ElectricSQL mapping for agent-editor and the initial integration plan (deferred until M4).

## Goals
- Local-first with CRDT replication for core knowledge graph entities.
- Deterministic derivations remain local (rebuildable), not replicated.
- No secrets or provider keys in replication stream.

## Tables and Policies

Replicate (CRDT):
- repo (LWW): `{ id, name, path, settings }`
- folder (LWW): `{ id, repo_id, parent_id, path, slug }`
- doc (LWW): `{ id, repo_id, folder_id, slug, title, lang, is_deleted, current_version_id, size_bytes, line_count, backlink_count }`
- doc_version (Append-only): `{ id, doc_id, blob_id, author, message, created_at, hash }`

Optional/Scoped replication:
- ai_trace (Append-only, off by default): `{ id, repo_id, doc_id, anchor_id, provider, request, response, … }`

Do not replicate (derived or local-only):
- link (derived from doc versions)
- doc_blob (content-addressed, large blobs; store via local cache or object store)
- provider (registry and settings are device-local; no keys)
- plugin, plugin_event (host-managed; keep local)
- app_setting (local UI state)

## Conflict Policies
- LWW for repo/folder/doc (timestamp ordering; ties resolved by node id).
- Append-only for doc_version and ai_trace (never delete/overwrite existing rows).
- Derived link table rebuilt on replica apply (background job).

## Initial Integration Plan
1) Create Electric mapping module (feature-flagged) that:
   - Exposes a "begin_replication()" facade (no-op until wired to Electric runtime).
   - Lists mapped tables and policies in code comments.
2) Add a small apply hook to rebuild `link` for changed docs after replication batches.
3) Skip large blob replication initially; reconstruct doc_fts from local doc_blob.
4) Add plan toggles in config (replicate_ai_traces=false by default).

## Testing
- Start with a local loopback (device A -> device B) using synthetic small repos.
- Validate invariants:
  - doc_version rows monotonic and hash unique per doc.
  - link derivation stable across nodes.
  - FTS doc_fts count == doc count after apply.

## Security
- Never replicate provider keys.
- Ensure `ai_trace.request/response` redactions are applied before persisting.

## Next Steps
- Implement feature flag `electric` to compile the mapping module.
- Provide a CLI `replica status` (later) to inspect applied ops and backlog.
