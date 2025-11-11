# Running Notes — agent-editor

Short, ongoing notes for open questions, ideas, and follow-ups. Keep entries concise.

## 2025-11-11 — Providers polish
- Clarified OpenRouter adapter usage (keys, model config, allowlist).
- Next: add provider troubleshooting to in-app Help tooltip on /settings/providers.

## 2025-11-11 — Plugin lifecycle
- Core plugins now auto-restart (3x, exp backoff) and log prefixed stderr/stdout lines to .sidecar.log.
- TODO: watchdog for call-core timeout honoring PLUGIN_CALL_TIMEOUT_MS; expose events to UI tail panel.

## 2025-11-11 — CI/perf benches
- ci:bench includes scan docs/sec assertions alongside FTS p95/p99/avg.
- Consider adding synthetic mixed-repo profile and environment-tuned thresholds.

## 2025-11-11 — Export docs
- Implemented `export_docs` RPC + CLI `agent-editor export docs` with repo filter, include-deleted flag, optional file output.
- Follow-up: implement `export db` (copy sqlite db) and support jsonl/tar formats per CLI plan.

## 2025-11-11 — Export db
- Added `export_db` RPC using SQLite backup to copy main DB to requested path; CLI `agent-editor export db --out <path>` now functional.
- Next: extend export formats (jsonl, tar) and include attachments when ready.

## 2025-11-11 — Export formats
- `agent-editor export docs --format jsonl --out docs.jsonl` writes newline-delimited JSON for streaming pipelines.
- `agent-editor export docs --format tar --out docs.tar --include-versions` builds a tarball containing `docs.json`, `versions.json`, and `meta.json`; attachments TODO.

## 2025-11-11 — Export tar tests
- Added Go unit test for writeDocsTar to ensure docs.json/meta.json integrity.
- Enforced --out requirement when format=jsonl|tar.
- Tar export now writes `docs/<slug-id>.md` files alongside docs.json/meta.json, providing raw Markdown snapshots.

## 2025-11-11 — CLI imports/tests
- Switched CLI imports to module path `github.com/agent-editor/agent-editor/cli/...` and fixed ai command braces so `go test ./cli/...` passes.

## 2025-11-11 — Import plan
- Added docs/plans/IMPORT_PLAN.md covering archive formats, CLI entrypoint, and next steps for import pipeline.

## 2025-11-11 — Import CLI stub
- Added `agent-editor import docs` CLI command that hits `import_docs` RPC (currently stubbed; validates --repo/--new-repo exclusivity).
- Documented archive format and import flow in `docs/manual/IMPORT.md`.

## 2025-11-11 — Import pipeline
- `import_docs` RPC now applies json/jsonl/tar archives, honors dry-run by default, and enforces merge strategies (keep/overwrite).
- New repo creation + folder scaffolding happen inside the import transaction; updates refresh FTS/link/provenance rows and attach a new version blob per doc.
- Added Rust round-trip tests plus CLI manual updates; next focus is attachment/blob handling once the binary-friendly doc_blob work lands.

## 2025-11-11 — Tar hydration
- Tar importer now hydrates missing doc bodies from `docs/<slug-id>.md`, matching the CLI’s filename sanitizer/truncation so long slugs resolve.
- Added regression test to ensure Markdown fallbacks stay wired; errors surface early if both docs.json and the tar snapshot omit content.

## 2025-11-11 — Import CLI defaults
- CLI `agent-editor import docs` now defaults to `--dry-run=true` to match the RPC/manual behavior.
- Guides updated to remind operators to pass `--dry-run=false` when applying mutations.

## 2025-11-11 — Import progress
- Added backend progress events every 25 docs plus final status; CLI tails a temporary log and prints `[import] STATUS processed/total inserted=…` lines in real time.
- Docs describe the streaming behavior; follow-up: consider piping structured progress to JSON output mode.

## 2025-11-11 — Import dedupe
- Overwrite imports now hash bodies and skip writing when content matches the current version, preventing redundant doc_version rows.
- Added regression test and docs/manual update so behavior stays visible.

## 2025-11-11 — Import attachments
- Tar archives can now include `attachments/<slug-id>/<filename>`; importer stores them in `doc_asset` with inferred MIME types and binary blobs.
- Added schema table, helper functions, and tests verifying attachment ingestion plus manual updates covering the format.

## 2025-11-11 — Export attachments
- CLI tar exports now include attachment files, and the RPC surfaces attachment metadata/base64 so other formats can consume them.
- Added Go tar test plus docs updates covering the archive layout.

## 2025-11-11 — Export attachments flag
- Added `--include-attachments` CLI flag so JSON/JSONL exports can opt-in to attachment payloads; tar still enables attachments automatically.

## 2025-11-11 — CLI tests
- `go test ./...` currently fails because the repo doesn’t have a `go.mod`; add a module before expanding Go-based test coverage.
