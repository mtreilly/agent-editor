#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-provider-demo"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }
tmux new-session -d -s "$SESSION" -n demo

# Pane 0: Sidecar
tmux send-keys -t "$SESSION":0.0 'cd src-tauri && cargo run --bin rpc_sidecar' C-m

# Pane 1: Build CLI; prepare temp repo; add and scan; enable openrouter; set key; set defaults; run ai
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'cd cli && go build -o agent-editor ./cmd/agent-editor && cd ..' C-m
tmux send-keys -t "$SESSION":0.1 'TMP=$(mktemp -d); echo "# Hello" > "$TMP/hello.md"; echo "Created temp repo: $TMP"' C-m
tmux send-keys -t "$SESSION":0.1 'printf "\n-- Add & scan --\n"; ./cli/agent-editor repo add "$TMP"; ./cli/agent-editor repo scan "$TMP"' C-m
tmux send-keys -t "$SESSION":0.1 'printf "\n-- Enable provider --\n"; ./cli/agent-editor ai providers enable openrouter' C-m
tmux send-keys -t "$SESSION":0.1 'printf "\n-- Set key --\n"; ./cli/agent-editor ai providers key set openrouter dummy_key' C-m
tmux send-keys -t "$SESSION":0.1 'printf "\n-- Set global default --\n"; ./cli/agent-editor settings default-provider set openrouter' C-m
tmux send-keys -t "$SESSION":0.1 'printf "\n-- Test provider --\n"; ./cli/agent-editor ai providers test openrouter' C-m
tmux send-keys -t "$SESSION":0.1 'printf "\n-- AI run (default) --\n"; ./cli/agent-editor ai run hello --provider default --prompt "Test"' C-m
tmux send-keys -t "$SESSION":0.1 'printf "\n-- AI run (explicit openrouter) --\n"; ./cli/agent-editor ai run hello --provider openrouter --prompt "Test"' C-m

# Pane 2: Logs
tmux split-window -v -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.2 'tail -f .sidecar.log' C-m

tmux select-layout -t "$SESSION":0 tiled
tmux attach -t "$SESSION"

