#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-bench"
REPO_PATH="${1:-}"
N_FILES="${2:-1000}"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

# Restart existing session for clean runs
tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }

tmux new-session -d -s "$SESSION" -n bench

# Pane 0: Sidecar (JSON-RPC)
tmux send-keys -t "$SESSION":0.0 'cd src-tauri && AE_DEBUG_SCAN=1 cargo run --bin rpc_sidecar' C-m

# Pane 1: FTS bench
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'pnpm bench:fts' C-m

# Pane 2: Scan bench (optional repo arg)
tmux split-window -v -t "$SESSION":0.0
if [[ -n "$REPO_PATH" ]]; then
  tmux send-keys -t "$SESSION":0.2 "bash scripts/bench-scan.sh '$REPO_PATH' '$N_FILES'" C-m
else
  tmux send-keys -t "$SESSION":0.2 "bash scripts/bench-scan.sh '' '$N_FILES'" C-m
fi

# Window 1: Logs
tmux new-window -t "$SESSION":1 -n logs
tmux send-keys -t "$SESSION":1.0 'tail -f .sidecar.log' C-m

tmux select-window -t "$SESSION":0
tmux select-layout -t "$SESSION":0 tiled
tmux attach -t "$SESSION"

