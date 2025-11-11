# CLI — agent-editor

A quick reference for the CLI. For a full map, see `docs/plans/CLI_PLAN.md`.

## Basics
- Binary: `agent-editor`
- Grammar: `agent-editor <resource> <action> [flags] [args]`
- Output flag: `-o json|yaml|text` (default: text)

## Common commands
```bash
# Repos
agent-editor repo add /abs/path --name notes
agent-editor repo scan /abs/path --watch --debounce 200
agent-editor repo list -o json
agent-editor repo info notes -o json
agent-editor repo remove notes --yes

# Docs
agent-editor doc create <repo-id> intro --title "Intro" --body "# Intro"
agent-editor doc update <doc-id> --body "New content" --message "edit"
agent-editor doc get <doc-id> --content -o json
agent-editor doc delete <doc-id> --yes

# Search & Graph
agent-editor doc search "query" -o json
agent-editor graph neighbors <doc-id> -o json
agent-editor graph path <start> <end> -o json

# Providers
agent-editor ai providers list -o json
agent-editor ai providers enable openrouter
agent-editor ai providers disable openrouter
agent-editor ai providers key set openrouter <key>
agent-editor ai providers key has openrouter -o json
agent-editor ai providers test openrouter -o json
agent-editor settings default-provider set openrouter

# Plugins (Core)
agent-editor plugin core-list -o json
agent-editor plugin start-core echo --exec node -- plugins/echo-core/echo.js
agent-editor plugin call-core echo '{"jsonrpc":"2.0","id":"1","method":"fs.read","params":{"path":"README.md"}}'
agent-editor plugin stop-core echo

# Plugins (Events)
agent-editor plugin events tail --file .sidecar.log --follow      # plugin-prefixed lines
agent-editor plugin events tail --all --from-beginning            # all lines, from start

# Export
agent-editor export docs --repo r1 --out docs.json
agent-editor export docs --include-deleted -o json
agent-editor export docs --out docs.jsonl --format jsonl
agent-editor export docs --out docs.tar --format tar --include-versions  # includes docs.json, versions.json, meta.json, docs/*.md
agent-editor export docs --out docs.json --include-attachments           # opt-in attachments for json outputs
agent-editor export db --out backup/agent-editor.db
```
> Tar exports also bundle `attachments/<slug-id>/<filename>` entries for any doc assets, so imports can restore binaries losslessly.

## Transport
- CLI talks to the JSON-RPC sidecar at `http://127.0.0.1:35678/rpc`.
- Start sidecar: `pnpm rpc:dev` (or run desktop with `pnpm dev`).

# Import
agent-editor import docs backup/docs.tar --repo r1 --dry-run
agent-editor import docs backup/docs.tar --new-repo imported --merge-strategy overwrite

> Dry-run mode is enabled by default; pass `--dry-run=false` when you are ready to apply the import.
> While the import runs the CLI streams `[import] …` progress lines (processed count + insert/update/skip stats) sourced from the backend progress log.
> Tar archives may include binary files under `attachments/<slug-id>/<filename>`; they are imported into each doc's attachment list automatically.
