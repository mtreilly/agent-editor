# Import Manual — agent-editor

This document describes the archive formats produced by `agent-editor export docs` and how the upcoming `agent-editor import docs` command will process them.

## Archive Layout (tar)
- `docs.json`: array of documents `{id, repo_id, slug, title, body, is_deleted, updated_at, versions?}`.
- `versions.json`: optional array `{doc_id, versions:[{id, hash, created_at, message}]}`.
- `docs/*.md`: raw Markdown snapshots named `docs/<slug>-<id>.md`.
- `meta.json`: summary metadata (`created_at`, `doc_count`, `version`).

## Import Command (planned)
```
agent-editor import docs <path> [--repo <id>|--new-repo <name>] [--dry-run] [--merge-strategy keep|overwrite]
```
- When `--repo` is provided, data is merged into an existing repo.
- `--new-repo` creates a fresh repo before import.
- `--dry-run` validates archives and reports diffs without touching the DB.
- `--merge-strategy` controls conflict handling:
  - `keep` (default) leaves existing docs untouched and reports conflicts.
  - `overwrite` replaces existing docs/versions.

## RPC Surface
- `import_docs` (stubbed): accepts `{path, repo_id?, new_repo_name?, dry_run?, merge_strategy?}`.
- Future implementation will stream archive contents, stage mutations in a transaction, and emit provenance records (`source='import'`).

## TODO
- Support json/jsonl inputs (without docs/*.md) by sourcing body directly from docs.json.
- Attachments/binary blobs once doc_blob stores non-Markdown content.
- CLI import progress events and summary reporting.
