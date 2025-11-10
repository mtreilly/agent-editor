# Status — agent-editor

Phase: M1 Core DB + Scanner + FTS (COMPLETE)

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

Exit criteria met
- repo add/scan works end-to-end with JSON-RPC and CLI
- Search returns valid JSON and matches FTS results (P95 on fixtures < 50ms)
- FTS invariant checks pass in `scripts/cli-smoke.sh` (fts_missing=0, errors=0)

Next Phase: M2 Editor + Wiki + Graph
- UI: search refinements (highlight snippets, keyboard nav), performance tweaks, settings
- Graph UI: depth controls, shortest path visualization, caching
- Editor: anchors list/jump-to/rename; copy link; improved wiki-link UX; i18n extraction for visible strings
- Parser hardening (wiki-link edge cases), more unit tests (non-ASCII, nested links, headings)
- Playwright: stabilize flaky selectors for Home/doc
- Sidecar: add DB trace + SQL logging for RPC handler around SEARCH; investigate post-dedupe SQL error path

Notes
- Tauri build requires a valid RGBA icon at `src-tauri/icons/icon.png` for packaging.
 - Approve SWC/esbuild builds with `pnpm approve-builds` if prompted.
