# Open Questions / TODO Later — agent-editor (Living Log)

Log uncertainties, pending decisions, and deferred work here. Keep items concise with context and proposed next experiments.

## Deferred (TODO Later)
- ElectricSQL integration (M4): implement mapping, feature flag `electric`, apply hooks for link/fts rebuild, CLI `replica status`.
- CI bench assertions: add script to validate FTS P95/P99 and scan throughput; fail CI if thresholds exceeded.
- UI a11y polish: audit routes with the A11Y checklist and add missing role/focus hints.
- Rustdoc headers: add module-level `//!` docs to `db.rs`, `commands.rs`, `scan/mod.rs`, `graph/mod.rs`.
- Provider UX: add per-repo provider selection E2E (real IPC) when running desktop; inform user of missing key via non-blocking toast.
- Plugin spawn polish: add timeouts, restart policy, and stdout/stderr logging to `.sidecar.log` with plugin name prefixes.
- CLI parity: add `repo update`, `export docs/db`, and `plugin events tail`.
- Packaging: multi-OS build matrix + codesigning instructions; add notarization notes for macOS.

## Open Questions
- Should we persist AI tokens usage and cost at the time of run (requires adapter-level token accounting)?
- Do we support streaming AI responses in UI (and how to record streaming in ai_trace)?
- What is the minimum viable set of provider models exposed in settings (per provider)?
- How do we want to surface plugin-provided scanners (UI/permissions/UX)?

## Notes
- When closing an item, link to the resolving commit/PR and docs update.
