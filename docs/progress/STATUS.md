# Status — agent-editor

Phase: M3 Plugins + Providers (IN PROGRESS)

Completed
- Core DB schema + FTS5 virtual table
- Tauri IPC commands + JSON-RPC localhost server
- CLI wired to JSON-RPC (repo/doc/search)
- Initial UI routes (home, search, repo, doc)
- Scanner pass: .gitignore-aware, imports *.md into DB with versions and FTS
- Wiki-link extraction: populate `link` on create/update and during scan
 - Graph APIs: neighbors/backlinks/related/path exposed over IPC/JSON-RPC and integrated in CLI/UI
 - Watcher-based incremental scan (notify) with debounce + progress events
- Headless JSON-RPC sidecar for dev automation
- RPC scan_repo wired to real scanner (CLI parity)
- CLI: FTS bench command (`fts bench`) + script `scripts/bench-fts.sh`
- Scanner dedupe: skip version/FTS when content hash unchanged; count only new/changed
- Folder slug: populate `folder.slug` from leaf name (spaces→dashes)
 - Milkdown editor + schema for wiki-links and anchor marks
- UI: doc graph/backlinks view; AI run with optional anchor context
 - UI: wiki-link navigation via editor; search results link to doc
 - Search: keyboard navigation, sanitized snippets, listbox ARIA + roving focus
- Graph: neighbor depth control (1–3) and reactive fetch
- Editor: anchors panel supports Jump and Copy Link; editor API exposes jumpToAnchor and anchorLinkFor
- Doc route: supports `?anchor=` param to auto-jump in editor
 - Parser: wiki-link extractor ignores escaped `\[\[`; tests for non-ASCII slugs, alias brackets, and unmatched opens
- AI: provider registry (SQLite) with privacy defaults; CLI and IPC for list/enable/disable wired
 - Packaging readiness: RGBA icon added; desktop uses AE_DB or .dev DB for builds/tests
- Benchmarks: `pnpm tmux:bench` orchestrates sidecar + FTS/scan benches; CLI `fts bench` reports avg/p50/p95/p99
- E2E: Graph path compute covered by Playwright with web IPC stubs; `pnpm tmux:e2e` runs dev + tests
- Settings: Providers management UI added (list/enable/disable) at /settings/providers, wired to IPC
 - i18n: Added i18next; extracted visible strings across nav, index, search, graph, editor (anchors/doc page), repo, and settings
 - A11y: Search listbox uses proper roles and aria-activedescendant; buttons/labels include ARIA where appropriate
 - E2E add: command palette toggle; settings/providers global default control; repos page default provider selector (web stubs)
- Plugins (UI/Core): minimal UI host to load Hello World plugin at /plugins and run a command; Core host scaffold in Rust
- Command Palette: Ctrl/Cmd+K palette uses plugin contributions; i18n and ARIA polish
- Providers: API key set/get stub via provider.config (to be replaced with OS keychain in M3)
 - Provider keys: keychain-ready facade added (`secrets.rs`), with keyring feature path and DB fallback storing only `key_set`
- Core plugins: spawn/stop RPC endpoints scaffolded (not implemented) and CLI wiring added
- Capability gate: call-core requires plugin.enabled=1 and permissions.core.call=true
 - Net domains: host enforces `permissions.net.domains` for `net.request*`; added tmux net demo
 - Permissions tests: unit tests added for core.call, net domains allowlist, fs roots allowlist (commands.rs tests)
- Providers (OpenRouter): adapter implemented with reqwest; model configurable; provider resolve IPC and UI gating; ai_run returns provider/model and UI shows header
- Repo page: shows effective provider (repo default or global)
- Packaging: tmux packaging script added (`pnpm tmux:tauri-build`); BUILD guide updated
- Docs: Added CODEMAP, PLUGINS, PROVIDERS, RPC, DATA_MODEL, DEVELOPMENT, CI, CONFIG, TROUBLESHOOTING, ROUTING, SCANNER, TESTING; README updated with links; IPC stubs manual.
 - Docs: Added CODEMAP, PLUGINS, PROVIDERS, RPC, DATA_MODEL, DEVELOPMENT, CI, CONFIG, TROUBLESHOOTING, ROUTING, SCANNER, TESTING; README updated with links; IPC stubs manual.
- Docs polish (M3): Enforced Vibe Discord notifications (start/progress/done) in AGENTS.md and added `guides/VIBE_NOTIFICATIONS.md`; scripts guide updated with headless fallback.
- CI/perf: `ci:bench` now asserts scan throughput (docs/sec) from `bench-scan.sh`; configurable via `SCAN_DOCS_PER_SEC_MIN`.
 - A11y polish: added aria-describedby/title hints on providers inputs; AI output region uses role=status + aria-live; repo scan progress uses aria-live.
 - E2E: providers hints and doc AI disabled describedby tested via Playwright.
 - CLI: `plugin events tail` to stream plugin-prefixed lines; docs updated.
- Plugins: restart policy + prefixed stderr/stdout logging; watchdog timeout via `PLUGIN_CALL_TIMEOUT_MS`; unit test using slow-core.
- Scripts: `tmux:plugin-log-smoke` to exercise logging and restart; packaging guide added with CI matrix.
 - CLI Export: `agent-editor export docs` (`--format json|jsonl|tar`, tar/jsonl require `--out`) + `agent-editor export db` wired to RPCs (docs snapshot + SQLite backup with `--out`). Added Go tar test.

Bench targets (current phase)
- FTS: P95 <= 50ms, P99 <= 80ms, avg <= 25ms on 100k docs synthetic dataset
- Scan throughput: >= 1,000 docs/sec on synthetic note set; >= 200 docs/sec on mixed repos

Exit criteria met (M1 + M2)
- M1: repo add/scan works end-to-end with JSON-RPC and CLI; search returns valid JSON and matches FTS results (P95 on fixtures < 50ms); FTS invariant checks pass in `scripts/cli-smoke.sh` (fts_missing=0, errors=0)
- M2: search UX (keyboard + ARIA + sanitized snippets); graph neighbors depth + path tool; editor anchors (insert/jump/copy) + `?anchor=` auto-jump; i18n extracted for core routes; providers registry + settings UI

Next Phase: M4 Packaging + CI
- UI Plugins: surface commands via host; integrate with command palette; plugin enable/disable lifecycle
- Core Plugins: wire spawn and JSON-RPC IPC; capability checks (FS/net/DB/AI)
- Providers: add real Codex/Claude/OpenRouter/OpenCode adapters; keychain storage; stricter redaction
- Parser hardening + more unit tests (nested links, non-ASCII, headings)
- E2E: add plugin UI smoke + provider selection tests (with stubs)

Notes
- Tauri build requires a valid RGBA icon at `src-tauri/icons/icon.png` for packaging.
 - Approve SWC/esbuild builds with `pnpm approve-builds` if prompted.
