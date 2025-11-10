# agent-editor — Design Philosophy & Quality Bar

Status: v0 (integrated from ~/vibe-engineering/docs/design)
Audience: Engineers, plugin authors, CLI/TUI developers

## North Star & Goals
- Speed to first edit/search: <150ms cold open to interactive editor.
- Zero-jank typing: keystroke-to-commit <8ms; no GC stalls in hot path.
- Instant search: FTS P95 <50ms on 100k docs; graph queries <30ms.
- SQLite is the single source of truth. No hidden state.
- Extensible and safe by default: capability-based plugins, network off by default.

## Architecture Principles (Quality Software)
- Single Source of Truth: SQLite holds everything (docs, versions, links, anchors, traces, settings). ElectricSQL syncs selected tables only.
- Layered & Resource-Oriented:
  - Core (Rust/Tauri) → DB + Services (Scanner, FTS, Graph, AI) → IPC → UI (React/Milkdown) → Plugins.
  - Prefer resource-first contracts (repo, folder, doc, link, plugin, scan_job).
- Contracts First:
  - Specify all IPC signatures, SQL DDL, triggers, and TS/Go types before coding. Backward-compatible changes only after M2.
- Deterministic & Idempotent:
  - Scanner upserts and dedupes via content-address (doc_blob.id) and unique constraints.
  - Triggers maintain invariants (FTS rows, backlink counts) atomically.
- Fast Path Priority:
  - Editor hot path avoids IPC chattiness; batch DB writes; use WAL; debounce watch events.
  - Code-split UI; lazy load heavy renderers (Mermaid, KaTeX, Shiki).
- Progressive Enhancement:
  - Local-first baseline; network providers optional; UI plugins optional.
- Observability Built-in:
  - Structured logs with levels; per-op timings; crash reports; privacy-safe.
- Capability Security:
  - Explicit grants (fs.read/write scoped, net domains, db.query RO/RW, ai.invoke providers). UI plugins get no FS direct access.
- Accessibility & Resilience:
  - Keyboard-first, ARIA complete; recover from panics; offline-robust; stale-while-revalidate for external data.

## CLI/TUI Design Principles (from vibe-engineering)
- Noun-Verb Command Model:
  - Pattern: `agent-editor <resource> <action> [flags] [args]` (e.g., `agent-editor repo scan`, `agent-editor doc search`).
  - Use singular resource names (repo, doc, plugin, graph, fts).
- Progressive Disclosure:
  - Simple defaults; advanced flags hidden unless `--help` or `-v`.
- Consistent Flags & Precedence:
  - `-o/--output text|json|yaml`, `-v/--verbose`, `--debug`, `-c/--config`.
  - Precedence: CLI > env > config file > defaults.
- Output Modes & Discoverability:
  - Human-friendly by default; `--json`/`--yaml` for automation.
  - Comprehensive help and examples per command; `--version` shows build info.
- Feedback & Safety:
  - Colors with NO_COLOR support; spinners/progress for long ops; `--dry-run` for destructive/bulk.
  - Confirm destructive ops; default to NO; `--yes` to bypass.
- TUI Ergonomics (Bubble Tea):
  - Never block Update(); async via tea.Cmd; debounce fast events; accessible (ASCII fallback, high contrast, configurable keybindings).
- Testing & Releases:
  - Unit tests for commands; golden tests for output; completions/man pages; GoReleaser for multi-OS.

## Resource Taxonomy & CLI Conventions
- Resources: repo, folder, doc, link, graph, fts, plugin, ai, serve, export.
- Examples:
  - `agent-editor repo add <path>` — register a repo.
  - `agent-editor repo scan <name|path> [--include ...] [--exclude ...] [--dry-run]`.
  - `agent-editor doc open <slug>` — open in desktop app or print path.
  - `agent-editor doc search <query> [-o json]` — structured results.
  - `agent-editor fts reindex [--repo <id>]` — rebuild index.
  - `agent-editor graph neighbors <doc-id> [--depth 2]`.
  - `agent-editor ai run <anchor-id> --provider <p> --prompt <file|text>`.
  - `agent-editor plugin install <name>` / `list` / `remove <name>`.
  - `agent-editor serve api [--port 35678]` — JSON-RPC bridge to desktop.
  - `agent-editor export docs [--repo <id>] [-o json]` — export metadata/content.

## Implementation Guardrails
- IPC/DB:
  - All IPC returns structured `{ code, message, details }` on error; long ops stream progress events.
  - Use WAL + NORMAL sync; transactions for batch ops; avoid N+1 writes.
- Editor:
  - ProseMirror decorations for anchors; milkdown schema for wiki-link/anchors; update DB on save only (no per-keystroke writes).
- Scanner:
  - `.gitignore` aware; content hash dedupe; incremental via mtime/hash; 200ms debounce.
- FTS/Graph:
  - External-content FTS5 table; triggers keep title/slug in sync; app updates body with UTF-8 text.
  - Graph queries via SQL CTEs; materialize doc_stats; invalidate on link changes.

## Testing & Quality
- Unit: slug rules, wiki-link parsing, triggers (backlinks/FTS), IPC contracts.
- Integration: scan→db→fts; plugin lifecycle; Electric sync conflict policies.
- TUI/CLI: golden outputs for text/json/yaml; completion generation; help includes examples.
- Performance: benchmark datasets; budgets enforced in CI.

## Versioning & Releases
- Desktop: Tauri 2 builds for macOS/Windows/Linux; codesign/signing as applicable.
- CLI: GoReleaser multi-arch; generate completions and man pages; embed version/commit/date via ldflags.

## References (source docs)
- ~/vibe-engineering/docs/design/CLI_DESIGN_PRINCIPLES.md
- ~/vibe-engineering/docs/design/CLI_PATTERNS_COOKBOOK.md
- agent-editor/docs/plans/MASTER_PLAN.md

