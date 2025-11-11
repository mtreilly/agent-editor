# Scanner — agent-editor

The scanner ingests Markdown files from configured repos and keeps the DB in sync.

## Pipeline (initial scan)
1) Build ignore set from `.gitignore` and internal defaults (node_modules, .git, dist).
2) Walk repo using `ignore` + `walkdir`, filter `**/*.md`.
3) For each file:
   - Compute relative path and slug (kebab-case; path separators → `/`).
   - Hash content (`blake3`) and build `{doc_id}:{content_hash}` version hash.
   - If unchanged vs current version, skip (dedupe).
   - Insert/Update: `folder`, `doc`, `doc_blob`, `doc_version`.
   - Maintain FTS (external-content): delete+insert doc_fts row under a single transaction.
   - Extract wiki-links and upsert `link` rows; update `backlink_count`.

## Watch mode
- Uses `notify` with debounce; rescans changed files; emits `progress.scan` events.

## Wiki-link extraction
- Ignores fenced/inline code and escaped `\\[\\[`.
- Supports alias syntax `[[slug|Alias]]`.
- Non-ASCII slugs supported; see tests in `graph/`.

## FTS
- FTS5 table `doc_fts` configured with `content_rowid='rowid'` and `content='doc'`.
- Updates are managed in app code (delete+insert) to keep determinism and portability.

## Invariants & tests
- `fts_missing` must be 0 after scans; smoke script enforces it.
- Dedupe ensures version/fts not updated if content hash unchanged.
- Unit tests cover slug generation and wiki-link extraction.
