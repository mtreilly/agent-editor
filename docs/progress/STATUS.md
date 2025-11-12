# Status — agent-editor

Phase: **M3 Plugins + Providers - COMPLETE** ✅

Completed M3 Work:
- ✅ Core DB schema + FTS5 virtual table
- ✅ Tauri IPC commands refactored into 11 feature modules
- ✅ Plugin spawn/shutdown lifecycle fully implemented
- ✅ JSON-RPC sidecar stable
- ✅ CLI wired to JSON-RPC (repo/doc/search/graph/ai/plugin)
- ✅ Wiki-link extraction with graph APIs
- ✅ Milkdown editor with anchors and wiki-links
- ✅ Search UX with keyboard navigation and ARIA
- ✅ Graph UI (neighbors, backlinks, path tool)
- ✅ AI providers: OpenRouter adapter with keychain storage
- ✅ Provider settings UI with enable/disable/keys
- ✅ Command palette with plugin contributions
- ✅ Plugin lifecycle: spawn_core_plugin(), shutdown_core_plugin(), call_core_plugin()
- ✅ Capability enforcement (fs, net, db, ai gates)
- ✅ Plugin restart policy with exponential backoff
- ✅ Comprehensive unit tests (14/14 passing, 100%)
- ✅ E2E tests (Playwright with web stubs)
- ✅ Import/export: docs (json/jsonl/tar) and database backup
- ✅ i18n extraction across all routes
- ✅ Code refactoring: commands.rs split from 2461 lines into 11 modules
- ✅ All compilation errors resolved
- ✅ Clippy clean (no major warnings)
- ✅ Documentation: M3_TESTING_REPORT.md created

Exit Criteria Met:
- ✅ M1: Repo add/scan works, FTS < 50ms P95
- ✅ M2: Editor functional, graph tools working, i18n extracted
- ✅ M3: Plugins spawn/call/shutdown, OpenRouter working, tests passing

Technical Debt Cleared:
- ✅ Monolithic commands.rs refactored into modules
- ✅ Plugin spawn abstraction implemented (no more direct OsCommand hacks)
- ✅ All module files < 500 lines (except justified export.rs)
- ✅ Clean module organization with feature-based structure

Next Phase: **M4 Sync + Packaging + Bench**

Notes
- Tauri build requires a valid RGBA icon at `src-tauri/icons/icon.png` for packaging.
 - Approve SWC/esbuild builds with `pnpm approve-builds` if prompted.
- Import CLI: now live; attachments/binary blobs will follow once doc blob storage supports non-Markdown payloads.
