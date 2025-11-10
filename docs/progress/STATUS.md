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

Pending
- Watcher-based incremental scan (notify) and debounced updates
- Milkdown editor + schema for wiki-links/anchors
- Graph APIs path/related
- UI: doc graph/backlinks view, search refinements

Notes
- Tauri build requires a valid RGBA icon at `src-tauri/icons/icon.png` for packaging.
