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
