# Status — agent-editor

Phase: M1 Core DB + Scanner + FTS (in progress)

Completed
- Core DB schema + FTS5 virtual table
- Tauri IPC commands + JSON-RPC localhost server
- CLI wired to JSON-RPC (repo/doc/search)
- Initial UI routes (home, search, repo, doc)
- Scanner pass: .gitignore-aware, imports *.md into DB with versions and FTS
- Wiki-link extraction: populate `link` on create/update and during scan
 - Graph APIs: neighbors/backlinks exposed over IPC/JSON-RPC and integrated in CLI/UI
 - Watcher-based incremental scan (notify) with debounce + progress events
 - Milkdown editor + schema for wiki-links and anchor marks
 - UI: doc graph/backlinks view; AI run with optional anchor context

Pending
- Graph APIs path/related
- UI: search refinements, performance tweaks, settings

Notes
- Tauri build requires a valid RGBA icon at `src-tauri/icons/icon.png` for packaging.
 - Approve SWC/esbuild builds with `pnpm approve-builds` if prompted.
