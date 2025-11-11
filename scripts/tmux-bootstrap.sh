#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-bootstrap"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }

tmux new-session -d -s "$SESSION" -n bootstrap

# Pane 0: pnpm install
tmux send-keys -t "$SESSION":0.0 'pnpm install' C-m

# Pane 1: cargo check (desktop)
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'cd src-tauri && cargo check' C-m

# Pane 2: CLI build
tmux split-window -v -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.2 'cd cli && go build ./cmd/agent-editor' C-m

tmux select-layout -t "$SESSION":0 tiled
tmux attach -t "$SESSION"

