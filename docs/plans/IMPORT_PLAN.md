# Import Plan — agent-editor

## Goals
- Support importing previously exported archives (json/jsonl/tar) into the local SQLite store.
- Preserve provenance and version metadata; avoid duplicate doc IDs when requested.
- Offer dry-run diffing before mutating.

## Formats
- `docs.json`: authoritative metadata, including repo_id, slug, title, body, deleted state, versions array.
- `versions.json`: optional per-doc version history (hash/message/timestamp).
- `docs/*.md`: raw markdown snapshots included in tar archives; source of truth if `docs.json[i].body` missing.

## Flow
1. Decompress archive or read jsonl stream.
2. Resolve repo target (existing or new) and slug conflicts.
3. For each doc:
   - Insert folder tree if missing.
   - Insert doc + doc_version rows; re-use provided IDs when possible.
   - Rebuild doc_fts + link table (re-run parser).
4. Emit provenance entries with `source='import'`.

## CLI
- `agent-editor import docs <path> [--repo <id>|--new-repo <name>] [--dry-run] [--merge-strategy keep|overwrite]`.
- Validate archive contents before touching DB; show summary diff.

## Open Questions
- Attachments/binaries: future work once doc blobs support binary MIME.
- Conflict strategy for existing slugs (default: fail).
- Access control for multi-user import (ElectricSQL phase).

## Next Steps
- ✅ CLI + RPC path implemented; docs/manual updated; round-trip tests cover json exports.
- ✅ Hydrate doc bodies from `docs/*.md` when `docs.json` omits content (tar fallback).
- ✅ Add attachment/blob support once binary-friendly doc_blob lands.
 - ✅ Stream progress + stats back to CLI (per ~25 docs) for large archives.
- ✅ Hash-based dedupe: overwrite imports compare version hashes and skip writing identical content.
