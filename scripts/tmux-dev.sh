#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-dev"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { echo "Session $SESSION already exists"; tmux attach -t "$SESSION"; exit 0; }

tmux new-session -d -s "$SESSION" -n main

# Pane 0: Sidecar (JSON-RPC)
tmux send-keys -t "$SESSION":0.0 'cd src-tauri && AE_DEBUG_SCAN=1 cargo run --bin rpc_sidecar' C-m

# Pane 1: Web dev server
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'pnpm dev:web' C-m

# Pane 2: Logs (sidecar)
tmux split-window -v -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.2 'tail -f .sidecar.log' C-m

# Pane 3: CLI smoke
tmux split-window -v -t "$SESSION":0.1
tmux send-keys -t "$SESSION":0.3 'bash scripts/cli-smoke.sh' C-m

tmux select-layout -t "$SESSION":0 tiled
tmux attach -t "$SESSION"

