# agent-editor — Master Plan (v0)

## Executive Summary
- Goal: agent-editor is a local-first Markdown knowledge system with instant editing/search, backlinks/graph, and line-level AI context. Desktop via Tauri 2, React 19 + TanStack Start, ElectricSQL for CRDT sync, and SQLite as the single source of truth.
- Constraints: cold-open to first edit <150ms; zero-jank typing; instant FTS; safe extensible plugins; no hidden state outside SQLite; greenfield.
- User journey: add repo(s) → scan obeying .gitignore → instant search across notes/code → edit Markdown with wiki-links, anchors, and block metadata → navigate link graph → trigger line-level AI context → extend with plugins and CLI/TUI.

## Design Philosophy & CLI Conventions
- See `docs/design/DESIGN_PHILOSOPHY.md` for the integrated design philosophy (from ~/vibe-engineering) covering architecture quality bar and CLI/TUI principles.
- Key tenets applied here:
  - Resource-first, noun-verb CLI (`agent-editor repo scan`, `agent-editor doc search`).
  - Consistent flags and precedence (CLI > env > config > defaults).
  - Human-friendly by default; `--json`/`--yaml` for automation; comprehensive help with examples.
  - Capability-based plugin security; SQLite as single source of truth; deterministic/idempotent ops.

## High-Level Architecture Diagram
```mermaid
flowchart TD
  subgraph Desktop[Tauri 2 App]
    FE[Frontend: React 19 + TanStack Start + Milkdown]
    IPC[Tauri IPC]
    CORE[Core Runtime (Rust)]
    PH[Plugin Host (Rust)]
  end

  subgraph Storage
    DB[(SQLite + FTS5)]
    EL[ElectricSQL (CRDT sync)]
  end

  subgraph Services
    SCN[Folder Scanner\n(.gitignore-aware)]
    FTS[FTS/Ranking Service]
    GR[Graph Service]
    AI[AI Connectors\n(Codex/Claude/OpenRouter/OpenCode)]
  end

  FE <---> IPC
  IPC <--> CORE
  CORE <--> DB
  CORE <--> SCN
  CORE <--> FTS
  CORE <--> GR
  CORE <--> PH
  FE <--> PH
  DB <--> EL
  CORE <--> AI

  subgraph DevHeadless
    Sidecar[RPC Sidecar]
  end
  Sidecar <--> DB
  Sidecar <--> FTS
  Sidecar <--> GR

  subgraph Plugins
    UIPlugin[UI Plugins (React)]
    CorePlugin[Core Plugins (Rust/Node/Go)]
  end

  PH <--> UIPlugin
  PH <--> CorePlugin
```

Host validation
- Validates JSON-RPC 2.0 envelope (jsonrpc, id, method), and enforces method-level permissions based on prefix:
  - `fs.write*` → `permissions.fs.write == true`
  - `fs.*` → `permissions.fs.read == true`
  - `net.request*` → `permissions.net.request == true`
  - `db.write*` → `permissions.db.write == true`
  - `db.*` → `permissions.db.query == true`
  - `ai.invoke*` → `permissions.ai.invoke == true`
  - `scanner.register*` → `permissions.scanner.register == true`
  - Always requires `permissions.core.call == true` and plugin `enabled == 1`

## Component Responsibilities & Boundaries
- Tauri Core (Rust)
  - Inputs: IPC commands; file system events.
  - Outputs: DB mutations; progress events; plugin bus messages.
  - Perf: command dispatch <1ms median; long ops streamed with progress.
- SQLite + FTS5
  - Inputs: normalized mutations, parsed links, versions, traces.
  - Outputs: transactional data, FTS results, graph queries.
  - Perf: FTS top-50 <10ms median on 100k docs.
- ElectricSQL
  - Inputs: CRDT ops for selected tables.
  - Outputs: multi-device sync with LWW/append-only policies.
  - Perf: apply op <5ms; idle CPU ~0.
- Folder Scanner
  - Inputs: repo roots, filters, .gitignore.
  - Outputs: repo/folder/doc rows; doc_version; link parse jobs.
  - Perf: initial 100k files <60s; incremental <200ms debounce.
- Graph Service
  - Inputs: link table.
  - Outputs: neighborhoods, related-docs, shortest-path.
  - Perf: neighborhood (k=2) <15ms; path <30ms on dense graphs.
- AI Connectors
  - Inputs: anchor IDs, context rules, provider settings.
  - Outputs: ai_trace; suggested edits; inline decorations.
  - Perf: assemble context <5ms; streaming responses.
- Frontend
  - Inputs: IPC queries; Electric subscriptions.
  - Outputs: user edits; plugin UI interactions.
  - Perf: first paint <150ms; keystroke-to-commit <4ms.
- Plugin Host
  - Inputs: plugin manifests; capability grants.
  - Outputs: commands, scanners, renderers, AI prompts.
  - Perf: plugin call overhead <2ms median.

## Data Model (SQLite)

### ERD
```mermaid
erDiagram
  repo ||--o{ folder : contains
  folder ||--o{ doc : contains
  doc ||--o{ doc_version : versions
  doc_version }o--|| doc_blob : content
  doc ||--o{ link : has
  doc ||--o{ ai_trace : traces
  repo ||--o{ scan_job : scans
  plugin ||--o{ plugin_event : emits
  doc ||--o{ provenance : provenance

  repo {
    TEXT id PK
    TEXT name
    TEXT path UNIQUE
    JSON settings
    TEXT created_at
    TEXT updated_at
  }
  folder {
    TEXT id PK
    TEXT repo_id FK
    TEXT parent_id
    TEXT path
    TEXT slug
    TEXT created_at
    TEXT updated_at
    UNIQUE (repo_id, path)
  }
  doc {
    TEXT id PK
    TEXT repo_id FK
    TEXT folder_id FK
    TEXT slug
    TEXT title
    TEXT lang
    INTEGER is_deleted
    TEXT current_version_id
    INTEGER size_bytes
    INTEGER line_count
    INTEGER backlink_count
    TEXT updated_at
    TEXT created_at
  }
  doc_version {
    TEXT id PK
    TEXT doc_id FK
    TEXT blob_id FK
    TEXT author
    TEXT message
    TEXT created_at
    TEXT hash UNIQUE
  }
  doc_blob {
    TEXT id PK  // content-addressable hash (e.g., blake3)
    BLOB content
    TEXT encoding  // utf8/zstd
    TEXT mime  // text/markdown
    INTEGER size_bytes
  }
  link {
    TEXT id PK
    TEXT repo_id FK
    TEXT from_doc_id FK
    TEXT to_doc_id  // nullable until resolved
    TEXT to_slug
    TEXT type  // wiki|url|heading|file
    INTEGER line_start
    INTEGER line_end
    TEXT created_at
    UNIQUE (from_doc_id, to_slug, line_start, line_end)
  }
  provenance {
    TEXT id PK
    TEXT entity_type  // doc|doc_version|link|ai_trace
    TEXT entity_id
    TEXT source  // fs|ai|import|plugin
    JSON meta
    TEXT created_at
  }
  scan_job {
    TEXT id PK
    TEXT repo_id FK
    TEXT status // queued|running|success|error|partial
    JSON stats
    TEXT started_at
    TEXT finished_at
    TEXT error
  }
  ai_trace {
    TEXT id PK
    TEXT repo_id FK
    TEXT doc_id
    TEXT anchor_id
    TEXT provider
    JSON request
    JSON response
    INTEGER input_tokens
    INTEGER output_tokens
    REAL cost_usd
    TEXT created_at
  }
  plugin {
    TEXT id PK
    TEXT name UNIQUE
    TEXT version
    TEXT kind  // ui|core
    JSON manifest
    JSON permissions
    INTEGER enabled
    TEXT installed_at
  }
  plugin_event {
    TEXT id PK
    TEXT plugin_id FK
    TEXT type
    JSON payload
    TEXT created_at
  }
```

### DDL
```sql
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS repo (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  settings JSON,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS folder (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  parent_id TEXT REFERENCES folder(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  slug TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (repo_id, path)
);

CREATE TABLE IF NOT EXISTS doc (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  folder_id TEXT NOT NULL REFERENCES folder(id) ON DELETE CASCADE,
  slug TEXT NOT NULL,
  title TEXT NOT NULL,
  lang TEXT DEFAULT 'en',
  is_deleted INTEGER NOT NULL DEFAULT 0,
  current_version_id TEXT REFERENCES doc_version(id),
  size_bytes INTEGER DEFAULT 0,
  line_count INTEGER DEFAULT 0,
  backlink_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (repo_id, slug)
);

CREATE TABLE IF NOT EXISTS doc_blob (
  id TEXT PRIMARY KEY, -- content hash
  content BLOB NOT NULL,
  encoding TEXT DEFAULT 'utf8',
  mime TEXT DEFAULT 'text/markdown',
  size_bytes INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS doc_version (
  id TEXT PRIMARY KEY,
  doc_id TEXT NOT NULL REFERENCES doc(id) ON DELETE CASCADE,
  blob_id TEXT NOT NULL REFERENCES doc_blob(id) ON DELETE RESTRICT,
  author TEXT,
  message TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  hash TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS link (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  from_doc_id TEXT NOT NULL REFERENCES doc(id) ON DELETE CASCADE,
  to_doc_id TEXT, -- resolved later
  to_slug TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('wiki','url','heading','file')),
  line_start INTEGER,
  line_end INTEGER,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (from_doc_id, to_slug, line_start, line_end)
);

CREATE TABLE IF NOT EXISTS provenance (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('fs','ai','import','plugin')),
  meta JSON,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS scan_job (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK (status IN ('queued','running','success','error','partial')),
  stats JSON,
  started_at TEXT NOT NULL DEFAULT (datetime('now')),
  finished_at TEXT,
  error TEXT
);

CREATE TABLE IF NOT EXISTS ai_trace (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  doc_id TEXT,
  anchor_id TEXT,
  provider TEXT NOT NULL,
  request JSON NOT NULL,
  response JSON,
  input_tokens INTEGER,
  output_tokens INTEGER,
  cost_usd REAL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plugin (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  version TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('ui','core')),
  manifest JSON NOT NULL,
  permissions JSON NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  installed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plugin_event (
  id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL REFERENCES plugin(id) ON DELETE CASCADE,
  type TEXT NOT NULL,
  payload JSON,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- FTS virtual table (external content)
CREATE VIRTUAL TABLE IF NOT EXISTS doc_fts USING fts5(
  title, body, slug, repo_id, content='doc', content_rowid='rowid',
  tokenize='unicode61 remove_diacritics 2'
);

-- FTS Triggers
CREATE TRIGGER IF NOT EXISTS doc_ai AFTER INSERT ON doc BEGIN
  INSERT INTO doc_fts(rowid, title, body, slug, repo_id)
  VALUES (new.rowid, new.title, '', new.slug, new.repo_id);
END;

CREATE TRIGGER IF NOT EXISTS doc_ad AFTER DELETE ON doc BEGIN
  INSERT INTO doc_fts(doc_fts, rowid) VALUES('delete', old.rowid);
END;

CREATE TRIGGER IF NOT EXISTS doc_au AFTER UPDATE OF title, slug, repo_id ON doc BEGIN
  INSERT INTO doc_fts(doc_fts, rowid) VALUES('delete', old.rowid);
  INSERT INTO doc_fts(rowid, title, body, slug, repo_id)
  VALUES (new.rowid, new.title, (SELECT body FROM doc_fts WHERE rowid=old.rowid), new.slug, new.repo_id);
END;

-- Keep body in FTS in sync with current version blob content
CREATE TRIGGER IF NOT EXISTS doc_version_ai AFTER INSERT ON doc_version BEGIN
  UPDATE doc SET current_version_id=new.id, updated_at=datetime('now') WHERE id=new.doc_id;
  -- Refresh FTS body from blob content (assumes utf8 text)
  INSERT INTO doc_fts(doc_fts, rowid) VALUES('delete', (SELECT rowid FROM doc WHERE id=new.doc_id));
  INSERT INTO doc_fts(rowid, title, body, slug, repo_id)
  SELECT d.rowid, d.title, CAST(hex(db.content) AS TEXT), d.slug, d.repo_id
  FROM doc d JOIN doc_blob db ON db.id=new.blob_id
  WHERE d.id=new.doc_id;
END;

-- Backlink counts maintenance on link changes
CREATE TRIGGER IF NOT EXISTS link_ai AFTER INSERT ON link WHEN new.to_doc_id IS NOT NULL BEGIN
  UPDATE doc SET backlink_count = backlink_count + 1 WHERE id=new.to_doc_id;
END;

CREATE TRIGGER IF NOT EXISTS link_ad AFTER DELETE ON link WHEN old.to_doc_id IS NOT NULL BEGIN
  UPDATE doc SET backlink_count = MAX(backlink_count - 1, 0) WHERE id=old.to_doc_id;
END;

-- Resolve links to doc ids on slug changes
CREATE TRIGGER IF NOT EXISTS doc_slug_au AFTER UPDATE OF slug ON doc BEGIN
  UPDATE link SET to_doc_id=new.id WHERE to_slug=new.slug AND repo_id=new.repo_id;
END;
```

Note: The `doc_version_ai` trigger stores blob content as hex to avoid encoding issues; in-app we update `doc_fts.body` directly with UTF-8 text when inserting versions.

## ElectricSQL Mapping
- Sync tables: `doc`, `doc_version`, `doc_blob` (optional), `link`, `folder`, `ai_trace` (opt-in), `plugin` (manifest only), `plugin_event` (opt-in).
- Local-only: `repo` (paths differ by machine), `scan_job`, `provenance` details beyond IDs.

Policies
- doc: LWW on scalar fields; `slug` unique per repo enforced via constraint.
- doc_version: append-only; unique by `hash`.
- doc_blob: content-addressed; unique PK prevents conflicts.
- link: LWW, unique composite prevents dupes; re-derived by parser on doc changes.
- folder: LWW with unique `(repo_id, path)`.
- ai_trace/plugin_event: append-only; can be excluded by privacy.

Example Electric schema + models
```ts
// src/electric/schema.ts
import { z } from 'zod'
import { electrify, TableConfig } from 'electric-sql/client'

export const Doc = z.object({
  id: z.string(),
  repo_id: z.string(),
  folder_id: z.string(),
  slug: z.string(),
  title: z.string(),
  lang: z.string(),
  is_deleted: z.number(),
  current_version_id: z.string().nullable(),
  size_bytes: z.number(),
  line_count: z.number(),
  backlink_count: z.number(),
  created_at: z.string(),
  updated_at: z.string(),
})
export type Doc = z.infer<typeof Doc>

export const tables: Record<string, TableConfig> = {
  doc: { replication: { strategy: 'lww' } },
  doc_version: { replication: { strategy: 'append_only' } },
  doc_blob: { replication: { strategy: 'key' } },
  link: { replication: { strategy: 'lww' } },
  folder: { replication: { strategy: 'lww' } },
  ai_trace: { replication: { strategy: 'append_only', enabled: false } },
  plugin: { replication: { strategy: 'lww' } },
  plugin_event: { replication: { strategy: 'append_only', enabled: false } },
}

export async function initElectric(db: any) {
  return electrify(db, { tables })
}
```

## Markdown/Wiki Pipeline
- Unified pipeline order:
  1) `remark-parse`
  2) `remark-gfm`
  3) `remark-frontmatter` (YAML/JSON/TOML)
  4) `remark-wiki-link` (custom mdast-util-wiki-link integration)
  5) `remark-smartypants`
  6) `remark-rehype` (allow raw)
  7) `rehype-katex` (math)
  8) `rehype-mermaidjs` (Mermaid render)
  9) `rehype-shiki` (code highlighting via Shiki)
  10) `rehype-stringify`

- mdast-util-wiki-link integration:
  - Syntax: `[[Slug|Alias]]`, `[[Slug#Heading]]`.
  - Parser maps to `wikiLink` node with `data: { slug, alias, heading }`.
  - Transformer: emits `link` nodes; also collects `link` rows for DB.

### Milkdown schema extensions
```ts
// src/editor/schema/wiki.ts
import { $node, $mark, $command, $inputRule } from '@milkdown/utils'

export const wikiLink = $node('wiki_link', () => ({
  inline: true,
  group: 'inline',
  atom: true,
  selectable: true,
  attrs: { slug: {}, alias: { default: null }, heading: { default: null } },
  toDOM: node => ['a', { 'data-slug': node.attrs.slug, class: 'wiki' }, node.attrs.alias || node.attrs.slug],
  parseDOM: [{ tag: 'a.wiki', getAttrs: el => ({ slug: (el as Element).getAttribute('data-slug') }) }],
}))

export const anchorMark = $mark('anchor', () => ({
  inclusive: false,
  attrs: { id: {} },
  parseDOM: [{ tag: 'span[data-anchor-id]' }],
  toDOM: mark => ['span', { 'data-anchor-id': mark.attrs.id }, 0],
}))

export const insertWikiLink = $command('insertWikiLink', ctx => (slug: string) => (state, dispatch) => {
  const { from, to } = state.selection
  const node = state.schema.nodes['wiki_link'].create({ slug })
  if (dispatch) dispatch(state.tr.replaceRangeWith(from, to, node))
  return true
})

export const wikiInputRule = $inputRule(/\[\[([^\]]+)\]\]$/, (state, match, start, end) => {
  const [full, inner] = match as unknown as [string, string]
  const [slug, alias] = inner.split('|')
  return state.tr.replaceWith(start, end, state.schema.nodes['wiki_link'].create({ slug, alias: alias || null }))
})
```

### Renderers
- KaTeX: `rehype-katex` with pre-bundled CSS; runs in frontend.
- Mermaid: `rehype-mermaidjs` runs client-side; in Tauri OK.
- Code highlighting: Shiki (WASM in UI). Preload popular themes to hit <150ms.

## Line-Level AI Context
- Decorations: ProseMirror decoration set with invisible `anchorMark` spans, IDs `anc_{docId}_{line}_{tsShort}`; zero‑width insertion to avoid layout shift.
- Persistence: anchors stored in `provenance` (`entity_type='anchor'`, `entity_id=<anchor_id>`, `meta: { doc_id, line }`). `ai_trace.anchor_id` references the ID.
- Context assembly:
  - Include N=12 lines around anchor; expand to enclosing fenced blocks or heading section; cap 2KB tokens by ellipsizing middle.
  - Include linked defs if anchor inside code fence and linkable identifier found.
- Safety and providers:
  - Redact secrets via regex allowlist + entropy scan.
  - Providers from `{ai_providers_default}`, network disabled by default; explicit opt-in per repo.

## Plugin Architecture
- Lifecycle: install → enable → onInit → onActivate(repo) → onDeactivate → uninstall.
- Sandboxing:
  - UI plugins: dynamic import, restricted API surface, no direct FS.
  - Core plugins: spawned as sidecar processes with seccomp-like fs/network allowlists; IPC only via Plugin Host.
- Capability grants:
  - fs.read, fs.write (scoped to whitelisted subpaths)
  - net.request (domain allowlist)
  - db.query (read-only or write)
  - ai.invoke (provider subset)
  - scanner.register(globs)
  - core.call (allow core plugin to receive call-core messages)
- Message bus (JSON-RPC 2.0):
```ts
// src/plugins/api.ts
export interface RpcRequest { jsonrpc: '2.0'; id: string; method: string; params?: any }
export interface RpcResponse { jsonrpc: '2.0'; id: string; result?: any; error?: { code: number; message: string; data?: any } }

export type Events =
  | { type: 'onDocSaved'; docId: string; versionId: string }
  | { type: 'onScanComplete'; repoId: string; jobId: string }
  | { type: 'onSearch'; query: string }
```

Host validation
- Validates JSON-RPC 2.0 envelope (jsonrpc, id, method), and enforces method-level permissions based on prefix:
  - `fs.write*` → `permissions.fs.write == true`
  - `fs.*` → `permissions.fs.read == true`
  - `net.request*` → `permissions.net.request == true`
  - `db.write*` → `permissions.db.write == true`
  - `db.*` → `permissions.db.query == true`
  - `ai.invoke*` → `permissions.ai.invoke == true`
  - `scanner.register*` → `permissions.scanner.register == true`
  - Always requires `permissions.core.call == true` and plugin `enabled == 1`
  - For `fs.*` methods, also enforces `permissions.fs.roots` allowlist: the `params.path` must be under an allowed root.

### Plugin API v1 (TypeScript)
```ts
// src/plugins/types.ts
export interface PluginContext {
  version: 'v1'
  permissions: Record<string, boolean>
  ipc: {
    call<T>(method: string, params?: any): Promise<T>
    on(event: string, handler: (payload: any) => void): () => void
  }
}

export interface Contributions {
  commands?: Array<{ id: string; title: string; run: (args: any) => Promise<void> }>
  views?: Array<{ id: string; mount: (el: HTMLElement, ctx: PluginContext) => void }>
  scanners?: Array<{ id: string; globs: string[]; handle: (file: { path: string; content: string }) => Promise<void> }>
  aiPrompts?: Array<{ id: string; title: string; build: (ctx: any) => Promise<string> }>
  renderers?: Array<{ type: 'node' | 'mark'; name: string; render: (node: any) => HTMLElement }>
}

export interface PluginV1 {
  name: string
  version: string
  kind: 'ui' | 'core'
  activate(ctx: PluginContext): Promise<Contributions>
  deactivate?(): Promise<void>
}
```

### Examples
```ts
// plugins/hello-world/index.ts
import type { PluginV1 } from '../../src/plugins/types'
const plugin: PluginV1 = {
  name: 'hello-world',
  version: '1.0.0',
  kind: 'ui',
  async activate(ctx) {
    return {
      commands: [{ id: 'hello.say', title: 'Say Hello', run: async () => alert('Hello!') }],
    }
  },
}
export default plugin
```

## Phase M3 — Plugins + AI Providers (Plan)

Goals
- Enable minimal plugin host: load one UI plugin and enumerate contributions; define core plugin spec and lifecycle (spawn/shutdown) with capability grants (scoped FS, network allowlist, DB RO/RW, AI provider allowlist).
- Integrate real AI providers with privacy defaults and key storage (OS keychain).

Scope
- UI Host (TS): dynamic import of UI plugins; aggregate contributions (commands/views/renderers); message bus to IPC.
- Core Host (Rust): spec struct for capabilities and exec; spawn (sidecar) process per plugin; JSON-RPC over stdin/stdout; permission checks in host.
- Providers: wire Codex, Claude Code, OpenRouter, OpenCode drivers; redact secrets; provider selection policy per repo.

Deliverables
- src/plugins/host.ts (UI) with `loadUIPlugin()` and `listUIContributions()`
- src-tauri/src/plugins/mod.rs (Core) with `Capabilities`, `CorePluginSpec`, and `spawn_core_plugin()` stub wired for future IPC
- Providers registry is active (v0); add keychain integration (M3 task)
- CLI: `plugin list|enable|disable|install|remove|call` (wire progressively to host APIs)

Exit Criteria
- Load at least one UI plugin (hello-world) and expose a command in app
- Spawn one core plugin in dry-run mode with logged capability grants
- Providers: at least one remote provider enabled behind opt-in with key loading (no network by default)

### Progress Update — OpenRouter Adapter
- Implemented OpenRouter provider call path in core (Rust) using reqwest with rustls (blocking client).
- Secrets fetched from OS keychain via `keyring` feature; DB fallback remains presence-only (no secret persisted).
- Endpoint allowlisted to `https://api.openrouter.ai/v1/chat/completions` for this adapter.
- `ai_run` now invokes the provider when `provider=openrouter` is enabled and key exists; on error, falls back to a clearly tagged echo stub.

### Progress Update — M3 E2E/QA
- Added Playwright tests (web stubs) for:
  - Command palette open + keyboard navigation with ARIA listbox semantics.
  - Settings: Providers page listing, enabling provider, and global default selection UX.
- Palette ARIA polish: `aria-activedescendant` on listbox and stable option ids.

### Progress Update — Packaging Prep
- Added tmux packaging script `pnpm tmux:tauri-build` to build frontend and package desktop app in separate panes; supports `HEADLESS=1`.
- BUILD guide updated to reference tmux packaging and headless usage.


```ts
// plugins/custom-scanner/index.ts
import type { PluginV1 } from '../../src/plugins/types'
const plugin: PluginV1 = {
  name: 'custom-scanner',
  version: '1.0.0',
  kind: 'core',
  async activate(ctx) {
    return {
      scanners: [{
        id: 'scan.todo',
        globs: ['**/*.md'],
        handle: async ({ path, content }) => {
          const todos = [...content.matchAll(/TODO:(.*)$/gm)].map(m => (m[1] || '').trim())
          await ctx.ipc.call('db.insertPluginEvent', { type: 'todoFound', payload: { path, todos } })
        }
      }]
    }
  }
}
export default plugin
```

## Folder Scanner
- Algorithm:
  1) Build ignore set from `.gitignore` and hardcoded ignores (`node_modules`, `.git`, `dist`).
  2) Discover files via `ignore` crate walker; filter `**/*.md`.
  3) For each file, compute slug: kebab-case of relative path without extension.
  4) If new or modified (mtime/hash), upsert `folder`, `doc`, `doc_blob`, `doc_version`.
  5) Enqueue parse for links; update `link` table; triggers update backlink counts.
  6) Start `notify` watcher (optional `--watch`); debounce 200–300ms; repeat for changes.

### Pseudocode + Tauri signature
```rust
#[derive(Deserialize)]
pub struct ScanFilters { pub include: Vec<String>, pub exclude: Vec<String> }

#[derive(Serialize)]
pub struct ScanJobReport { pub job_id: String, pub files_scanned: i64, pub docs_added: i64, pub errors: i64 }

#[tauri::command]
pub async fn scan_repo(repo_path: String, filters: Option<ScanFilters>, watch: Option<bool>, debounce: Option<u64>) -> Result<ScanJobReport, String> {
    // 1) insert scan_job (running)
    // 2) initial scan via ignore + walkdir; upsert rows; update FTS; compute stats
    // 3) update scan_job (success) + stats
    // 4) if watch: spawn notify watcher with debounce; emit `progress.scan` events
}
```

### Mirroring rules
- repo.path is absolute; folder.path is repo-relative; slug = relative path no extension with `/` → `__` for hierarchy safe slugs; ID = `blake3(repo_id + ':' + slug)`.

## Search & Graph

### FTS5 queries (prefix, phrase, boolean)
```sql
SELECT d.id, d.slug, bm25(doc_fts, 1.2, 0.75) AS rank,
  snippet(doc_fts, 1, '<b>','</b>','…', 8) AS title_snip,
  snippet(doc_fts, 2, '<b>','</b>','…', 8) AS body_snip
FROM doc_fts
JOIN doc d ON d.rowid = doc_fts.rowid
WHERE doc_fts MATCH :query AND d.repo_id = :repo_id
ORDER BY rank ASC, d.updated_at DESC
LIMIT :limit OFFSET :offset;
```

- Ranking signals: BM25 + recency boost + backlink_count weight: `rank' = rank - log1p(backlink_count)*0.2 - recency_weight`.

### Backlinks/graph API
- Neighborhood query
```sql
SELECT l2.from_doc_id AS neighbor_id FROM link l
JOIN link l2 ON l.to_doc_id = l2.to_doc_id
WHERE l.from_doc_id = :doc_id AND l2.from_doc_id != :doc_id
UNION
SELECT to_doc_id FROM link WHERE from_doc_id = :doc_id AND to_doc_id IS NOT NULL;
```

- Shortest path (recursive CTE)
```sql
WITH RECURSIVE
  q(n, path) AS (
    SELECT :start_id, json_array(:start_id)
    UNION ALL
    SELECT CASE WHEN l.to_doc_id IS NULL THEN '' ELSE l.to_doc_id END, json_insert(q.path, '$[#]', l.to_doc_id)
    FROM q JOIN link l ON l.from_doc_id = q.n
    WHERE json_array_length(q.path) < 12 AND l.to_doc_id NOT IN (SELECT value FROM json_each(q.path))
  )
SELECT path FROM q WHERE json_extract(path, '$[#-1]') = :end_id LIMIT 1;
```

### Cached materialized views and invalidation
- `doc_stats(doc_id, backlinks, outlinks, updated_at)` maintained by triggers on `link`.
- Invalidate on `link` insert/delete.
 
### Return types
- GraphDoc: `{ id: string; slug: string; title: string }`
- IPC/JSON-RPC: `graph_neighbors(doc_id, depth?) -> GraphDoc[]`, `graph_backlinks(doc_id) -> GraphDoc[]`

## Tauri 2 IPC + Frontend Contracts

### Rust commands (signatures)
```rust
#[tauri::command] async fn repos_add(path: String, name: Option<String>, include: Option<Vec<String>>, exclude: Option<Vec<String>>) -> Result<serde_json::Value, String>;
#[tauri::command] async fn repos_list() -> Result<Vec<serde_json::Value>, String>;
#[tauri::command] async fn repos_info(id_or_name: String) -> Result<serde_json::Value, String>;
#[tauri::command] async fn repos_remove(id_or_name: String) -> Result<serde_json::Value, String>;
#[tauri::command] async fn scan_repo(repo_path: String, filters: Option<ScanFilters>, watch: Option<bool>, debounce: Option<u64>) -> Result<ScanJobReport, String>;

#[derive(Deserialize)] struct DocCreate { repo_id: String, slug: String, title: String, body: String }
#[tauri::command] async fn docs_create(payload: DocCreate) -> Result<serde_json::Value, String>;
#[derive(Deserialize)] struct DocUpdate { doc_id: String, body: String, message: Option<String> }
#[tauri::command] async fn docs_update(payload: DocUpdate) -> Result<serde_json::Value, String>;
#[tauri::command] async fn docs_get(doc_id: String, content: Option<bool>) -> Result<serde_json::Value, String>;
#[tauri::command] async fn docs_delete(doc_id: String) -> Result<serde_json::Value, String>;

#[derive(Serialize)] struct SearchHit { id: String, slug: String, title_snip: String, body_snip: String, rank: f64 }
#[tauri::command] async fn search(repo_id: Option<String>, query: String, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<SearchHit>, String>;

#[derive(Serialize)] struct GraphDoc { id: String, slug: String, title: String }
#[tauri::command] async fn graph_neighbors(doc_id: String, depth: Option<u8>) -> Result<Vec<GraphDoc>, String>;
#[tauri::command] async fn graph_backlinks(doc_id: String) -> Result<Vec<GraphDoc>, String>;

#[tauri::command] async fn ai_run(provider: String, doc_id: String, anchor_id: Option<String>, prompt: String) -> Result<serde_json::Value, String>;
#[tauri::command] async fn anchors_upsert(doc_id: String, anchor_id: String, line: i64) -> Result<serde_json::Value, String>;
#[tauri::command] async fn serve_api_start(port: Option<u16>) -> Result<(), String>;
```

### Implemented commands (v0)
- Repo: `repos_add`, `repos_list`, `repos_info`, `repos_remove`, `scan_repo`
- Docs: `docs_create`, `docs_update`, `docs_get`, `docs_delete`
- Search: `search` (FTS5 with bm25 + snippet)
- Graph: `graph_backlinks`, `graph_neighbors`, `graph_related`, `graph_path`
- AI: `ai_run` (local echo stub, context assembly, ai_trace persistence)
- Anchors: `anchors_upsert`, `anchors_list`, `anchors_delete`
- Providers: `ai_providers_list`, `ai_providers_enable`, `ai_providers_disable` (privacy defaults)
- Sidecar control: `serve_api_start`

### TS client wrappers
Headless JSON-RPC sidecar for automation:
- Run: `cargo run --manifest-path src-tauri/Cargo.toml --bin rpc_sidecar`
- Endpoint: `http://127.0.0.1:35678/rpc` (used by CLI/tests)
```ts
// src/ipc/client.ts
import { invoke } from '@tauri-apps/api/tauri'
export const reposAdd = (path: string, name?: string, include?: string[], exclude?: string[]) =>
  invoke<{ repo_id: string }>('repos_add', { path, name, include, exclude })
export const reposList = () => invoke<Array<{ id: string; name: string; path: string }>>('repos_list')
export const reposInfo = (id_or_name: string) => invoke<any>('repos_info', { idOrName: id_or_name })
export const reposRemove = (id_or_name: string) => invoke<{ removed: boolean }>('repos_remove', { idOrName: id_or_name })

export const scanRepo = (repoPath: string, filters?: { include?: string[]; exclude?: string[] }, watch?: boolean, debounce?: number) =>
  invoke<{ job_id: string; files_scanned: number; docs_added: number; errors: number }>('scan_repo', { repoPath, filters, watch, debounce })

export const docsCreate = (repo_id: string, slug: string, title: string, body: string) =>
  invoke<{ doc_id: string }>('docs_create', { payload: { repo_id, slug, title, body } })
export const docsUpdate = (doc_id: string, body: string, message?: string) =>
  invoke<{ version_id: string }>('docs_update', { payload: { doc_id, body, message } })
export const docsGet = (doc_id: string, content?: boolean) => invoke<any>('docs_get', { docId: doc_id, content })
export const docsDelete = (doc_id: string) => invoke<{ deleted: boolean }>('docs_delete', { docId: doc_id })

export type SearchHit = { id: string; slug: string; title_snip: string; body_snip: string; rank: number }
export const search = (query: string, repo_id?: string, limit = 50, offset = 0) =>
  invoke<SearchHit[]>('search', { repoId: repo_id, query, limit, offset })

export type GraphDoc = { id: string; slug: string; title: string }
export const graphBacklinks = (doc_id: string) => invoke<GraphDoc[]>('graph_backlinks', { docId: doc_id })
export const graphNeighbors = (doc_id: string, depth = 1) => invoke<GraphDoc[]>('graph_neighbors', { docId: doc_id, depth })

export const aiRun = (provider: string, doc_id: string, prompt: string, anchor_id?: string) =>
  invoke<{ trace_id: string; text: string }>('ai_run', { provider, docId: doc_id, anchorId: anchor_id, prompt })
export const anchorsUpsert = (doc_id: string, anchor_id: string, line: number) =>
  invoke<{ ok: boolean }>('anchors_upsert', { docId: doc_id, anchorId: anchor_id, line })
```

Browser fallback (tests): Wrap `invoke` with a guard that returns empty stubs when not running in Tauri. This allows Playwright/web-only tests to render UI without desktop runtime.

- Error model: commands return `Result<Ok, ErrString>`. Error strings are structured JSON: `{ code, message, details }`. Long ops stream progress via `tauri::async_runtime::spawn` and window `emit("progress.scan", {...})`.

## Routing (TanStack Start)
- Routes: `/` (home), `/search`, `/doc/:id`, `/repo`, `/settings`, `/plugins`, `/graph/:id`
- Vite plugin config: `TanStackRouterVite({ routesDirectory: 'app/routes', generatedRouteTree: 'app/routeTree.gen.ts' })` to generate route tree for type-safe routing.
- SSR strategy:
  - Desktop: client-only hydration; data via IPC.
  - Web build (dev docs): SSR enabled for static pages; dynamic routes client-only.

## Performance Budgets & Benchmarks
- Cold start: <150ms to interactive editor (cached code-split, pre-init DB/IPC during splash).
- First edit apply-to-DB: <8ms commit; <4ms keystroke-to-decoration.
- Search latency (100k docs): <50ms P95 for prefix/phrase; boolean <80ms.
- Bench plan:
  1) Dataset: 100k Markdown notes, avg 1.2KB; 10 repos mix; generate fixtures.
  2) Scenarios: cold launch, open doc, save doc, FTS prefix/phrase/boolean, neighbor graph.
  3) Scripts: Rust criterion benches for FTS/graph; Playwright for UI timings; Go CLI for scan throughput; CLI `fts bench` for latency (avg, p50, p95, p99).
  4) Example: `agent-editor fts bench --query "foo" --n 50 -o json`
  5) Orchestrate via tmux: `pnpm tmux:bench` (Pane A: sidecar, Pane B: FTS bench, Pane C: scan bench)

Enforcement
- Target thresholds:
  - FTS P95 <= 50ms on 100k docs; P99 <= 80ms; avg <= 25ms (prefix/phrase).
  - Scan throughput >= 1,000 docs/sec on SSD (initial pass on synthetic dataset), >200 docs/sec on large mixed repos.
- Run `pnpm tmux:bench` and record results in docs/progress/STATUS.md each milestone.

## Security & Privacy
- Local-first, network off by default.
- Providers: opt-in per repo; api keys stored in OS keychain; providers restricted by allowlist.
- Plugin sandbox: deny-by-default; explicit capability grants; per-plugin FS chroot to repo or cache; network domain allowlist.
- Secrets scanning and redaction before AI calls; logs redact by default.

### Provider Key Storage
- Primary: OS keychain via keyring crate (feature `keyring`). Keys never persisted in SQLite.
- Fallback (no keyring): store a `key_set` boolean in `provider.config` (no secret material stored) and read key from environment when invoking provider.
- IPC:
  - `ai_provider_key_set(name, key)` — writes to keychain (or sets `key_set=1`)
  - `ai_provider_key_get(name)` — returns `{ has_key: boolean }` only
  - Future: `ai_provider_key_clear(name)` to remove key

## Phase Checkpoints

M1 Core DB + Scanner + FTS — COMPLETE
- End-to-end: repo add/scan, search JSON results with FTS parity
- FTS invariants verified (fts_missing=0) by `scripts/cli-smoke.sh`
- Sidecar JSON-RPC stable; packaging readiness (RGBA icon, AE_DB/.dev DB)

M2 Editor + Wiki + Graph — COMPLETE
- Search UX: keyboard navigation, sanitized snippets, listbox ARIA with roving focus
- Graph UI: neighbor depth control (1–3), shortest path tool, e2e smoke
- Editor: anchors insert/jump/copy link; doc route supports `?anchor=` auto-jump
- i18n: extracted strings (common, search, graph, editor, settings, repo)
- Providers: registry + settings UI to enable/disable (network off by default)

## Build/Run/Package
- Dev: `pnpm install && pnpm dev` (starts Vite + Tauri dev)
- Build: `pnpm build` (Vite) then `pnpm tauri build`
- Note: Ensure a valid RGBA icon at `src-tauri/icons/icon.png`. Approve SWC/esbuild builds if prompted: `pnpm approve-builds`.
- Test: `pnpm test`, `pnpm test:e2e`
- Packaging:
  - Targets: `{mac_win_linux}`
  - macOS: codesign with developer ID; hardened runtime
  - Windows: MSIX signing; Linux: AppImage + DEB/RPM
- Env:
  - Root dir: `{root_dir}`
  - Scan roots: `{abs_paths_or_globs}`
  - Default providers enabled: `{ai_providers_default}`

## CLI/TUI Companion (Go)
- Detailed command contracts live in `docs/plans/CLI_PLAN.md` and follow the problem → correctness → solution → proof structure.
- Tooling: Cobra, Viper, Charm (Bubble Tea + Lip Gloss), Colang for grammar parsing.

### Wire protocol
- IPC over localhost: `http://127.0.0.1:35678` JSON-RPC (Tauri sidecar server).

### Commands
```go
// cmd/root.go
package cmd
import "github.com/spf13/cobra"
var rootCmd = &cobra.Command{ Use: "agent-editor", Short: "CLI/TUI for agent-editor" }
func Execute() { _ = rootCmd.Execute() }
```
```go
// cmd/repo_scan.go
package cmd
import "github.com/spf13/cobra"
var repoScanCmd = &cobra.Command{
  Use: "repo scan [path]",
  RunE: func(cmd *cobra.Command, args []string) error {
    path := args[0]
    _ = path
    // TODO: POST /rpc scan_repo
    return nil
  },
}
func init() { rootCmd.AddCommand(repoScanCmd) }
```
```go
// tui/app.go
package tui
import tea "github.com/charmbracelet/bubbletea"

type model struct { query string; results []string }
func (m model) Init() tea.Cmd { return nil }
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) { return m, nil }
func (m model) View() string { return "Search: " + m.query }
```

- Command set (noun-verb):
  - `repo add <path>`, `repo scan <path|name>`, `repo list`, `repo info <name>`
  - `doc open <slug>`, `doc search <query> [-o json]`, `doc export [--repo <id>]`
  - `fts reindex [--repo <id>]`
  - `graph neighbors <doc-id> [--depth 2]`, `graph related <doc-id>`
  - `ai run <anchor-id> --provider <p> --prompt <text|file>`
  - `plugin install <name>`, `plugin list`, `plugin remove <name>`
  - `serve api [--port 35678]`
  - `export docs [--repo <id>] [-o json]`

## Testing Strategy
- Unit: markdown parsing, wiki-link extraction, slug rules, FTS triggers, link count triggers.
- Integration: scan→db→fts flow; plugin command execution; Electric sync on doc/link.
- UI smoke: Playwright at 320/768/1024/1440 for editor input, search, navigation, plugins view.
- Plugin conformance: run plugin test harness to validate permissions and API shape.
 - E2E additions: graph path tool smoke; doc page panels with web-only IPC stubs. CLI smoke script exercises repo add/scan/search/graph.
 - i18n: extraction check — no hardcoded user-facing strings in components; validate locale keys exist; fallback coverage.
 - Plugins: UI host smoke — load hello-world plugin and execute a command; Core host dry-run spawn (logs capability grants).

## Milestones
1) M1 Core DB + Scanner + FTS (Exit: scan repo, search works; 100k docs index <60m, search <80ms P95)
2) M2 Editor + Wiki + Graph (Exit: edit + backlinks update; graph neighbors; line anchors persisted)
3) M3 Plugins + AI Connectors (Exit: Hello World UI/Core plugins; AI run with anchors; privacy defaults)
4) M4 Sync + Packaging + Bench (Exit: Electric sync enabled; installers for `{mac_win_linux}`; benchmarks meet targets)

## Risks & Mitigations
- FTS latency at scale → ahead-of-time tokenization and per-repo FTS shards.
- Large repos scan time → incremental hashing + watch; pause/resume.
- Plugin security → strict capability model; isolate by process; audit logs.
- CRDT conflicts on slugs → deterministic slug merge + rename queues.
- Binary size → code-splitting; shared libs; strip symbols.
- KaTeX/Mermaid perf → virtualize render; lazy load.
- ElectricSQL service availability → optional local-only mode; graceful degrade.
- Tauri IPC bottlenecks → batch DB ops; stream progress; avoid chatty calls.

## Appendices

### File Tree
```
agent-editor/
├── src/
│   ├── app/                 # TanStack Start routes
│   ├── editor/              # Milkdown, schema, commands
│   ├── ipc/                 # TS wrappers
│   ├── electric/            # schema, init
│   ├── plugins/             # host, types
│   ├── features/
│   │   ├── search/
│   │   ├── graph/
│   │   └── settings/
├── src-tauri/
│   ├── src/
│   │   ├── db.rs
│   │   ├── scan.rs
│   │   ├── fts.rs
│   │   ├── graph.rs
│   │   ├── ai.rs
│   │   └── plugins.rs
│   └── tauri.conf.json
├── plugins/
│   ├── hello-world/
│   └── custom-scanner/
├── cli/
│   ├── cmd/
│   └── tui/
└── docs/
```

### OpenAPI/TS Types (excerpt)
```ts
// src/types/api.ts
export interface SearchHit { id: string; slug: string; title_snip: string; body_snip: string; rank: number }
export interface Doc { id: string; repo_id: string; folder_id: string; slug: string; title: string }
export interface GraphEdge { from: string; to: string; type: 'wiki'|'url'|'heading'|'file' }
```

### Config Schema (JSON-Schema)
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Agent Editor Config",
  "type": "object",
  "properties": {
    "providers": {
      "type": "object",
      "additionalProperties": { "type": "object" }
    },
    "repos": {
      "type": "array",
      "items": { "type": "object", "properties": {
        "name": { "type": "string" },
        "path": { "type": "string" },
        "include": { "type": "array", "items": { "type": "string" } },
        "exclude": { "type": "array", "items": { "type": "string" } }
      }, "required": ["name","path"] }
    }
  }
}
```

### Example Datasets and Fixtures
- 10k Markdown notes with `[[links]]`, headings, code fences.
- Synthetic repo with nested folders and mixed `.md` and ignored paths.
- Provider stubs that echo prompts for deterministic tests.

---

## Seed Inputs
- Root dir: `{root_dir}`
- Scan roots: `{abs_paths_or_globs}`
- Default providers enabled: `{ai_providers_default}`
- Platform targets: `{mac_win_linux}`

---

## GAP REPORT
- SSR details for TanStack Start inside Tauri are simplified to client-only; propose adding a small Node runner for web targets only.
- ElectricSQL exact runtime init depends on chosen adapter; finalize during implementation.
- `doc_fts.body` hex insert in trigger should be replaced by app-side update to ensure UTF-8 correctness; include a post-version-insert app routine to refresh FTS body with text.
