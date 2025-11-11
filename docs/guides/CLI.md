# CLI — agent-editor
# CLI — agent-editor


A quick reference for the CLI. For a full map, see `docs/plans/CLI_PLAN.md`.
A quick reference for the CLI. For a full map, see `docs/plans/CLI_PLAN.md`.


## Basics
## Basics
- Binary: `agent-editor`
- Binary: `agent-editor`
- Grammar: `agent-editor <resource> <action> [flags] [args]`
- Grammar: `agent-editor <resource> <action> [flags] [args]`
- Output flag: `-o json|yaml|text` (default: text)
- Output flag: `-o json|yaml|text` (default: text)


## Common commands
## Common commands
```bash
```bash
# Repos
# Repos
agent-editor repo add /abs/path --name notes
agent-editor repo add /abs/path --name notes
agent-editor repo scan /abs/path --watch --debounce 200
agent-editor repo scan /abs/path --watch --debounce 200
agent-editor repo list -o json
agent-editor repo list -o json
agent-editor repo info notes -o json
agent-editor repo info notes -o json
agent-editor repo remove notes --yes
agent-editor repo remove notes --yes


# Docs
# Docs
agent-editor doc create <repo-id> intro --title "Intro" --body "# Intro"
agent-editor doc create <repo-id> intro --title "Intro" --body "# Intro"
agent-editor doc update <doc-id> --body "New content" --message "edit"
agent-editor doc update <doc-id> --body "New content" --message "edit"
agent-editor doc get <doc-id> --content -o json
agent-editor doc get <doc-id> --content -o json
agent-editor doc delete <doc-id> --yes
agent-editor doc delete <doc-id> --yes


# Search & Graph
# Search & Graph
agent-editor doc search "query" -o json
agent-editor doc search "query" -o json
agent-editor graph neighbors <doc-id> -o json
agent-editor graph neighbors <doc-id> -o json
agent-editor graph path <start> <end> -o json
agent-editor graph path <start> <end> -o json


# Providers
# Providers
agent-editor ai providers list -o json
agent-editor ai providers list -o json
agent-editor ai providers enable openrouter
agent-editor ai providers enable openrouter
agent-editor ai providers key set openrouter <key>
agent-editor ai providers key set openrouter <key>
agent-editor settings default-provider set openrouter
agent-editor settings default-provider set openrouter


# Plugins (Core)
# Plugins (Core)
agent-editor plugin core-list -o json
agent-editor plugin core-list -o json
agent-editor plugin start-core echo --exec node -- plugins/echo-core/echo.js
agent-editor plugin start-core echo --exec node -- plugins/echo-core/echo.js
agent-editor plugin call-core echo '{"jsonrpc":"2.0","id":"1","method":"fs.read","params":{"path":"README.md"}}'
agent-editor plugin call-core echo '{"jsonrpc":"2.0","id":"1","method":"fs.read","params":{"path":"README.md"}}'
agent-editor plugin stop-core echo
```

# Plugins (Events)
agent-editor plugin events tail --file .sidecar.log --follow      # plugin-prefixed lines
agent-editor plugin events tail --all --from-beginning            # all lines, from start

# Export (stubs)
agent-editor export docs   # stub
agent-editor export db     # stub

```
```
```


## Transport
## Transport
- CLI talks to the JSON-RPC sidecar at `http://127.0.0.1:35678/rpc`.
- CLI talks to the JSON-RPC sidecar at `http://127.0.0.1:35678/rpc`.
- Start sidecar: `pnpm rpc:dev` (or run desktop with `pnpm dev`).
- Start sidecar: `pnpm rpc:dev` (or run desktop with `pnpm dev`).
