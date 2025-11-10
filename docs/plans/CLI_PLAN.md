# agent-editor — CLI Plan (v0)

References: docs/plans/MASTER_PLAN.md, docs/design/DESIGN_PHILOSOPHY.md
Rule of exposition: 1) Problem, 2) Correctness conditions, 3) Solution spec, 4) Proof/tests.

## Global CLI
- Binary: `agent-editor`
- Grammar: `agent-editor <resource> <action> [flags] [args]`
- Resources: `repo`, `doc`, `fts`, `graph`, `ai`, `plugin`, `serve`, `export`, `config`, `completion`, `version`
- Global flags:
  - `-v, --verbose` (counting: `-vv`)
  - `--debug`
  - `-o, --output text|json|yaml` (default: text)
  - `-c, --config <path>`
  - `--yes` (assume yes for prompts)
  - `--dry-run` (no side effects)
  - `--no-color`
  - `--version` (also `version` subcommand)
- Output modes: text (human) by default; JSON/YAML structured.
- Exit codes: 0 OK; 2 usage; 10 config; 11 network; 12 auth; 13 db; 14 not_found; 15 permission; 16 invalid_state; 130 Ctrl+C.
- Transport: JSON-RPC over `http://127.0.0.1:35678` (Tauri sidecar) or direct Tauri IPC when running embedded. All commands map to RPC methods listed below.

## RPC Method Map (CLI → RPC)
- repo.add → `repos_add`
- repo.scan → `scan_repo`
- repo.list/info/remove/update → `repos_list|repos_info|repos_remove|repos_update` (to be added)
- doc.create/update/get/delete → `docs_create|docs_update|docs_get|docs_delete`
- doc.search → `search`
- fts.reindex/query/stats → `fts_reindex|fts_query|fts_stats`
- graph.neighbors/path/related/backlinks → `graph_neighbors|graph_path|graph_related|graph_backlinks`
- ai.run → `ai_run`, ai.providers.* → `ai_providers_list|enable|disable`
- plugin.list/call/install/remove/enable/disable/info/events.tail → `plugins_list|plugins_call|plugins_install|plugins_remove|plugins_enable|plugins_disable|plugins_info|plugins_events_tail`
- serve.api → `serve_api_start`
- export.docs/db → `export_docs|export_db`
- config.* → local files; no RPC

## Resource: repo

### repo add <path>
- Problem: Register a repo root for scanning and indexing.
- Correctness: Path exists; not already registered; store absolute path and generated repo_id; idempotent for same path.
- Solution: `agent-editor repo add /abs/path --name <name> [--include "**/*.md"] [--exclude "node_modules/**"]`
- Proof/tests: Adding same path twice returns same repo_id; invalid path → exit 2; record in `repo` table.

### repo scan <path|name>
- Problem: Index Markdown files, obeying .gitignore, update DB.
- Correctness: Only included files; excluded per filters; progress events; atomic batches; no duplicates.
- Solution: `agent-editor repo scan <path|name> [--include ...] [--exclude ...] [--watch] [--debounce 200ms] [--dry-run]`
- Proof/tests: Dry-run shows planned counts; watch emits changes; DB updated; `scan_job` row created.

### repo list
- Problem: Discover configured repos.
- Correctness: Returns stable ordering; prints name, id, path.
- Solution: `agent-editor repo list [-o json]`
- Proof/tests: JSON schema validates; empty list handled.

### repo info <name|id>
- Problem: Inspect a repository’s details.
- Correctness: Returns settings, stats; not_found → exit 14.
- Solution: `agent-editor repo info <name|id>`
- Proof/tests: Matches DB content; includes last scan status.

### repo remove <name|id>
- Problem: Unregister repo without deleting data on disk.
- Correctness: Removes `repo` row and cascades; confirm unless `--yes`.
- Solution: `agent-editor repo remove <name|id> [--yes]`
- Proof/tests: Subsequent `repo info` → not_found; data removed from DB.

## Resource: doc

### doc create <repo> <slug>
- Problem: Create a new document with initial body.
- Correctness: Unique slug per repo; version row created; FTS updated.
- Solution: `agent-editor doc create <repo-id|name> <slug> --title <title> [--body "..."] [--file <path>]`
- Proof/tests: FTS returns the new title/body; link triggers update on subsequent saves.

### doc update <doc-id|slug>
- Problem: Save a new version.
- Correctness: Appends `doc_version`; updates `doc.current_version_id`; FTS refreshed.
- Solution: `agent-editor doc update <doc-id|slug> [--repo <id>] [--message <msg>] [--file <path>|--body "..."]`
- Proof/tests: Version count increments; FTS snippets reflect new content.

### doc get <doc-id|slug>
- Problem: Retrieve metadata/content.
- Correctness: Output matches DB blob; respects `-o` mode.
- Solution: `agent-editor doc get <doc-id|slug> [--repo <id>] [--content] [-o json|text]`
- Proof/tests: JSON has blob hash and size; text prints content.

### doc delete <doc-id|slug>
- Problem: Soft delete a document.
- Correctness: `doc.is_deleted=1`; content retained; backlinks unaffected.
- Solution: `agent-editor doc delete <doc-id|slug> [--repo <id>] [--yes]`
- Proof/tests: Search excludes deleted by default.

### doc search <query>
- Problem: Full-text search.
- Correctness: Scoped by repo/folder/tag; FTS queries with BM25; snippets.
- Solution: `agent-editor doc search <query> [--repo <id>] [--limit 50] [--offset 0] [-o json]`
- Proof/tests: Matches MASTER_PLAN SQL; P95 <50ms on fixtures.

## Resource: fts

### fts query <query>
- Problem: Low-level FTS access.
- Correctness: Exact parity with `doc search` core; raw ranking/snippets.
- Solution: `agent-editor fts query <query> [--repo <id>] [--limit 50] [-o json]`
- Proof/tests: Identical results as `doc search` with same params.

### fts reindex
- Problem: Rebuild FTS.
- Correctness: Non-destructive; consistent after completion.
- Solution: `agent-editor fts reindex [--repo <id>] [--vacuum]`
- Proof/tests: Post-reindex queries identical; timing reported.

### fts stats
- Problem: Inspect index stats.
- Correctness: Returns row counts, sizes.
- Solution: `agent-editor fts stats [--repo <id>] [-o json]`
- Proof/tests: Values match DB PRAGMAs.

## Resource: graph

### graph neighbors <doc-id>
- Problem: Local link neighborhood.
- Correctness: Includes 1..depth hops; dedup; excludes self.
- Solution: `agent-editor graph neighbors <doc-id> [--depth 2] [-o json]`
- Proof/tests: SQL matches MASTER_PLAN query; bounded depth.

### graph path <start-id> <end-id>
- Problem: Shortest path along links.
- Correctness: Returns one minimal-length path; max-depth enforced.
- Solution: `agent-editor graph path <start-id> <end-id> [--max-depth 12]`
- Proof/tests: Returns empty when none; complexity bounded.

### graph related <doc-id>
- Problem: Related documents ranking.
- Correctness: Uses co-citation and link-overlap; deterministic.
- Solution: `agent-editor graph related <doc-id> [--k 20]`
- Proof/tests: Stable ranking on fixtures.

### graph backlinks <doc-id>
- Problem: Inbound links.
- Correctness: Accurate counts; resolved only.
- Solution: `agent-editor graph backlinks <doc-id>`
- Proof/tests: Equals `doc.backlink_count`.

## Resource: ai

### ai providers list|enable|disable
- Problem: Manage AI providers.
- Correctness: No network by default; keys stored in OS keychain.
- Solution: `agent-editor ai providers list|enable <name>|disable <name>`
- Proof/tests: Provider state persists; unsafe providers disabled by default.

### ai run <doc|anchor>
- Problem: Run AI with line-level context.
- Correctness: Context assembly rules; redaction; trace persisted.
- Solution: `agent-editor ai run <doc-id|slug> [--anchor <id>|--line <n>] --provider <p> --prompt <text|@file>`
- Proof/tests: Trace stored; response streamed; secrets redacted.

### ai traces list
- Problem: Inspect traces.
- Correctness: Paginates; structured output.
- Solution: `agent-editor ai traces list [--doc <id>] [-o json]`
- Proof/tests: Counts match `ai_trace`.

## Resource: plugin

### plugin install <name|path>
- Problem: Install a plugin.
- Correctness: Manifest validation; sandboxed; capability grants.
- Solution: `agent-editor plugin install <name|path>`
- Proof/tests: Appears in `plugin` table; disabled if permissions missing.

### plugin list|info|remove|enable|disable|call|events tail
- Solution:
  - `agent-editor plugin list`
  - `agent-editor plugin info <name>`
  - `agent-editor plugin remove <name> [--yes]`
  - `agent-editor plugin enable <name>` / `disable <name>`
  - `agent-editor plugin call <name> <method> [--params-json <json>]`
  - `agent-editor plugin events tail [--name <name>]`
- Proof/tests: CRUD affects `plugin`/`plugin_event`; call routed via JSON-RPC with permissions checked.

## Resource: serve

### serve api
- Problem: Expose CLI-accessible API for automation.
- Correctness: Binds localhost only; auth token optional; health endpoint.
- Solution: `agent-editor serve api [--port 35678]`
- Proof/tests: Health check passes; rejects non-localhost by default.

## Resource: export

### export docs
- Problem: Export documents and metadata.
- Correctness: Reproducible snapshots; includes versions optionally.
- Solution: `agent-editor export docs [--repo <id>] [--format jsonl|tar] [--out <path>]`
- Proof/tests: Schema validates; re-import produces identical rows.

### export db
- Problem: Backup SQLite.
- Correctness: Uses online backup; WAL-safe.
- Solution: `agent-editor export db [--out <path>]`
- Proof/tests: File opens; pragma integrity_check OK.

## Resource: config

### config init|get|set|path
- Problem: Manage CLI config.
- Solution:
  - `agent-editor config init`
  - `agent-editor config get <key>`
  - `agent-editor config set <key> <value>`
  - `agent-editor config path`
- Proof/tests: XDG paths respected; precedence order maintained.

## Resource: completion & version

### completion generate
- Problem: Shell completions.
- Solution: `agent-editor completion generate <bash|zsh|fish>`
- Proof/tests: Generated files source correctly.

### version
- Problem: Show build info.
- Solution: `agent-editor version`
- Proof/tests: Shows semver, commit, date.

## Minimal Working Skeleton (Cobra, single-file)
```go
// cmd/agent-editor/main.go
package main

import (
    "fmt"
    "os"
    "github.com/spf13/cobra"
)

var (
    output  string
    verbose int
    debug   bool
    yes     bool
    dryRun  bool
)

func main() {
    root := &cobra.Command{Use: "agent-editor", Short: "Local-first Markdown knowledge system"}
    root.PersistentFlags().CountVarP(&verbose, "verbose", "v", "verbose output (-v, -vv)")
    root.PersistentFlags().BoolVar(&debug, "debug", false, "debug mode")
    root.PersistentFlags().StringVarP(&output, "output", "o", "text", "output format (text|json|yaml)")
    root.PersistentFlags().BoolVar(&yes, "yes", false, "assume yes for prompts")
    root.PersistentFlags().BoolVar(&dryRun, "dry-run", false, "do not make changes")

    // repo
    repo := &cobra.Command{Use: "repo", Short: "Manage repositories"}
    repoAdd := &cobra.Command{Use: "add <path>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub repo add", args[0]); return nil }}
    repoScan := &cobra.Command{Use: "scan <path|name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub repo scan", args[0]); return nil }}
    repoList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub repo list"); return nil }}
    repoInfo := &cobra.Command{Use: "info <name|id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub repo info", args[0]); return nil }}
    repoRemove := &cobra.Command{Use: "remove <name|id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub repo remove", args[0]); return nil }}
    repo.AddCommand(repoAdd, repoScan, repoList, repoInfo, repoRemove)

    // doc
    doc := &cobra.Command{Use: "doc", Short: "Document operations"}
    docCreate := &cobra.Command{Use: "create <repo> <slug>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub doc create", args); return nil }}
    docUpdate := &cobra.Command{Use: "update <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub doc update", args[0]); return nil }}
    docGet := &cobra.Command{Use: "get <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub doc get", args[0]); return nil }}
    docDelete := &cobra.Command{Use: "delete <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub doc delete", args[0]); return nil }}
    docSearch := &cobra.Command{Use: "search <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub doc search", args[0]); return nil }}
    doc.AddCommand(docCreate, docUpdate, docGet, docDelete, docSearch)

    // fts
    fts := &cobra.Command{Use: "fts", Short: "Full-text search ops"}
    ftsQuery := &cobra.Command{Use: "query <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub fts query", args[0]); return nil }}
    ftsReindex := &cobra.Command{Use: "reindex", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub fts reindex"); return nil }}
    ftsStats := &cobra.Command{Use: "stats", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub fts stats"); return nil }}
    fts.AddCommand(ftsQuery, ftsReindex, ftsStats)

    // graph
    graph := &cobra.Command{Use: "graph", Short: "Link graph operations"}
    graphNeighbors := &cobra.Command{Use: "neighbors <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub graph neighbors", args[0]); return nil }}
    graphPath := &cobra.Command{Use: "path <start-id> <end-id>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub graph path", args); return nil }}
    graphRelated := &cobra.Command{Use: "related <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub graph related", args[0]); return nil }}
    graphBacklinks := &cobra.Command{Use: "backlinks <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub graph backlinks", args[0]); return nil }}
    graph.AddCommand(graphNeighbors, graphPath, graphRelated, graphBacklinks)

    // ai
    ai := &cobra.Command{Use: "ai", Short: "AI operations"}
    aiRun := &cobra.Command{Use: "run <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub ai run", args[0]); return nil }}
    aiProviders := &cobra.Command{Use: "providers", Short: "Manage AI providers"}
    aiProvidersList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub ai providers list"); return nil }}
    aiProvidersEnable := &cobra.Command{Use: "enable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub ai providers enable", args[0]); return nil }}
    aiProvidersDisable := &cobra.Command{Use: "disable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub ai providers disable", args[0]); return nil }}
    aiTraces := &cobra.Command{Use: "traces", Short: "Inspect AI traces"}
    aiTracesList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub ai traces list"); return nil }}
    aiProviders.AddCommand(aiProvidersList, aiProvidersEnable, aiProvidersDisable)
    aiTraces.AddCommand(aiTracesList)
    ai.AddCommand(aiRun, aiProviders, aiTraces)

    // plugin
    plugin := &cobra.Command{Use: "plugin", Short: "Plugin management"}
    pluginInstall := &cobra.Command{Use: "install <name|path>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin install", args[0]); return nil }}
    pluginList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin list"); return nil }}
    pluginInfo := &cobra.Command{Use: "info <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin info", args[0]); return nil }}
    pluginRemove := &cobra.Command{Use: "remove <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin remove", args[0]); return nil }}
    pluginEnable := &cobra.Command{Use: "enable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin enable", args[0]); return nil }}
    pluginDisable := &cobra.Command{Use: "disable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin disable", args[0]); return nil }}
    pluginCall := &cobra.Command{Use: "call <name> <method>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin call", args); return nil }}
    pluginEvents := &cobra.Command{Use: "events", Short: "Plugin events"}
    pluginEventsTail := &cobra.Command{Use: "tail", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub plugin events tail"); return nil }}
    pluginEvents.AddCommand(pluginEventsTail)
    plugin.AddCommand(pluginInstall, pluginList, pluginInfo, pluginRemove, pluginEnable, pluginDisable, pluginCall, pluginEvents)

    // serve
    serve := &cobra.Command{Use: "serve", Short: "Local services"}
    serveAPI := &cobra.Command{Use: "api", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub serve api"); return nil }}
    serve.AddCommand(serveAPI)

    // export
    export := &cobra.Command{Use: "export", Short: "Export data"}
    exportDocs := &cobra.Command{Use: "docs", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub export docs"); return nil }}
    exportDB := &cobra.Command{Use: "db", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub export db"); return nil }}
    export.AddCommand(exportDocs, exportDB)

    // config
    config := &cobra.Command{Use: "config", Short: "CLI configuration"}
    configInit := &cobra.Command{Use: "init", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub config init"); return nil }}
    configGet := &cobra.Command{Use: "get <key>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub config get", args[0]); return nil }}
    configSet := &cobra.Command{Use: "set <key> <value>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub config set", args); return nil }}
    configPath := &cobra.Command{Use: "path", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub config path"); return nil }}
    config.AddCommand(configInit, configGet, configSet, configPath)

    // completion
    completion := &cobra.Command{Use: "completion", Short: "Generate shell completions"}
    completionGen := &cobra.Command{Use: "generate <bash|zsh|fish>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("stub completion generate", args[0]); return nil }}
    completion.AddCommand(completionGen)

    // version
    version := &cobra.Command{Use: "version", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("agent-editor dev (commit: none, built: unknown)"); return nil }}

    root.AddCommand(repo, doc, fts, graph, ai, plugin, serve, export, config, completion, version)

    if err := root.Execute(); err != nil {
        fmt.Fprintln(os.Stderr, err)
        os.Exit(1)
    }
}
```

## Testing Strategy (CLI)
- Golden tests for text and JSON outputs per command (`--output` variations).
- Exit-code tests for error paths (usage, not_found, permission).
- Contract conformance vs RPC schemas; offline tests using a stub RPC server.
- Completions generation smoke tests for bash/zsh/fish.

