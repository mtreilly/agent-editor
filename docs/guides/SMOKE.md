# Smoke Test — agent-editor

Goal: validate JSON-RPC, scanning, and CLI without launching the Tauri UI.

## Prerequisites
- Rust/Cargo installed
- Go 1.22+
- pnpm

## Steps
1) Start headless JSON-RPC sidecar

```
pnpm rpc:dev
```

2) In a new terminal, run the CLI smoke

```
pnpm smoke:cli
```

This script will:
- Build the CLI
- Create a temporary repo with a couple markdown files
- Call `repo add`, `repo scan`, and `doc search`

## Manual quick test
```
# Terminal A: start sidecar
pnpm rpc:dev

# Terminal B: build CLI and run quick commands
(cd cli && go build ./cmd/agent-editor -o agent-editor)
./cli/agent-editor repo list -o json
```

Notes
- The sidecar uses an on-disk dev DB under `src-tauri/.dev/`.
- For desktop UI with IPC, run `pnpm dev` instead of the sidecar.
- The CLI expects the RPC at `http://127.0.0.1:35678` (configurable via viper `server`).
