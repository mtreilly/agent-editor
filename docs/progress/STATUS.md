# Status — agent-editor

Phase: M1 Core DB + Scanner + FTS (nearing completion)

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

Pending
- UI: search refinements, performance tweaks, settings
- Router plugin generated route tree types (remove shim)
- Playwright flaky selectors fix for Home/doc
- Parser hardening (wiki-link edge cases), more unit tests
 - Investigate sidecar SEARCH SQL error post-dedupe (web/e2e unaffected; IPC route OK). Add DB trace + SQL logging for RPC handler.

Notes
- Tauri build requires a valid RGBA icon at `src-tauri/icons/icon.png` for packaging.
 - Approve SWC/esbuild builds with `pnpm approve-builds` if prompted.
