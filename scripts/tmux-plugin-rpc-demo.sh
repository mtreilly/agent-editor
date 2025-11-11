#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-plugin-rpc-demo"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }
tmux new-session -d -s "$SESSION" -n demo

# Pane 0: Sidecar
tmux send-keys -t "$SESSION":0.0 'cd src-tauri && cargo run --bin rpc_sidecar' C-m

# Pane 1: CLI build + setup perms + spawn core
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'cd cli && go build -o agent-editor ./cmd/agent-editor && cd ..' C-m
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin perms set echo --json '{\"core\":{\"call\":true}}'" C-m
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin enable echo" C-m
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin start-core echo --exec node -- plugins/echo-core/echo.js" C-m
sleep 1
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin call-core echo '{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"method\":\"fs.read\",\"params\":{\"path\":\"README.md\"}}'" C-m
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin perms set echo --json '{\"core\":{\"call\":true},\"fs\":{\"read\":true,\"roots\":[\".\"]}}'" C-m
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin call-core echo '{\"jsonrpc\":\"2.0\",\"id\":\"2\",\"method\":\"fs.read\",\"params\":{\"path\":\"README.md\"}}'" C-m

# Pane 2: Logs
tmux split-window -v -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.2 'tail -f .sidecar.log' C-m

tmux select-layout -t "$SESSION":0 tiled
tmux attach -t "$SESSION"
