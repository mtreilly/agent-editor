# Import Manual — agent-editor

This guide documents the archive formats produced by `agent-editor export docs` and how the importer applies them through `agent-editor import docs`.

## Archive Layout
- `docs.json` — authoritative metadata `{id, repo_id, slug, title, body, is_deleted, updated_at, versions?}`.
- `versions.json` — optional `{doc_id, versions:[{id, hash, created_at, message}]}` bundles.
- `docs/*.md` — raw Markdown snapshots (`docs/<slug>-<id>.md`) used when `body` is omitted.
- `attachments/<slug-id>/<filename>` — optional binary assets imported into `doc_asset` (slug-id matches the prefix used for `docs/<slug-id>.md`).
- `meta.json` — summary metadata (`created_at`, `doc_count`, `version`).
- `json` / `jsonl` inputs skip the tar container but must include the same document fields.

## Command
```
agent-editor import docs <path> [--repo <id> | --new-repo <name>] \
  [--merge-strategy keep|overwrite] [--dry-run]
```
- Exactly one of `--repo` or `--new-repo` is required. The latter creates the repo (and root folder) before import.
- `--dry-run` (default) parses the archive, simulates inserts/updates, and reports stats without mutating SQLite.
- `--merge-strategy keep` (default) skips conflicts, `overwrite` updates existing docs and writes a new version.

## Execution Flow
1. Read the archive (json/jsonl/tar) and materialize `DocImportRow` entries.
2. Resolve the repo target:
   - existing repo (`--repo`) must exist;
   - new repo path stored under `.import/<slugified-name>`.
3. For tar archives, hydrate missing doc bodies by loading the matching `docs/<slug-id>.md` snapshot (slug sanitation matches exporter rules, so long names/characters resolve correctly). If the Markdown file is missing, the import fails early with a helpful error.
4. Attachments under `attachments/<slug-id>/<filename>` are imported into `doc_asset` (deduped by filename per doc) and stored as blobs with inferred MIME types.
5. For dry runs, count inserts/updates/skips via `simulate_import`.
6. For real imports, run a single DB transaction that:
   - inserts or updates `doc`, `doc_version`, and `doc_blob`;
   - refreshes `doc_fts` rows and rebuilds wiki-link edges;
   - writes provenance records with `source='import'` and `{path}` metadata.

All mutations occur under the same transaction to keep FTS, versions, and provenance in sync.

The CLI automatically creates a temporary progress log; the backend appends JSON events every ~25 docs so the CLI can display `[import] PROCESSING …` lines while the import runs.

## Response Payload
Both dry runs and real imports return a structured summary (printed by the CLI):

```json
{
  "path": "/tmp/docs.tar",
  "doc_count": 42,
  "repo_id": "repo_x",
  "created_repo": false,
  "merge_strategy": "keep",
  "dry_run": false,
  "inserted": 40,
  "updated": 1,
  "skipped": 1,
  "status": "imported"
}
```

`status` is `dry_run` or `imported`. When `--new-repo` is provided, `created_repo` reports whether a repo was inserted.

## Merge Strategy
- `keep`: existing `repo+slug` rows are left untouched and counted under `skipped`.
- `overwrite`: existing docs are updated in place, a new version/blob is created, FTS rows are rebuilt, and provenance is recorded. When the incoming body matches the current version hash, the importer skips the update to avoid redundant versions (counts under `skipped`).

## Current Limitations / Next Work
- CLI progress events currently stream as text only; JSON output mode doesn’t include progress updates yet.
