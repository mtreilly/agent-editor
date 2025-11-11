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
