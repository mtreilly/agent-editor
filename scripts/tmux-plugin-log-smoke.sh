#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-plugin-log"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }

tmux new-session -d -s "$SESSION" -n demo

# Pane 0: Sidecar
tmux send-keys -t "$SESSION":0.0 'cd src-tauri && cargo run --bin rpc_sidecar' C-m

# Pane 1: CLI build + start echo core + make a call, then simulate crash and call again (restart policy)
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'cd cli && go build -o agent-editor ./cmd/agent-editor && cd ..' C-m
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin start-core echo --exec node -- plugins/echo-core/echo.cjs" C-m
sleep 1
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin call-core echo '{\"jsonrpc\":\"2.0\",\"id\":\"1\",\"method\":\"fs.read\",\"params\":{\"path\":\"README.md\"}}'" C-m

# Simulate unexpected exit to exercise auto-restart on next call
tmux send-keys -t "$SESSION":0.1 "PID=\$(./cli/agent-editor plugin core-list -o json | node -e 'let d=\"\";process.stdin.on(\"data\",c=>d+=c).on(\"end\",()=>{try{let j=JSON.parse(d);console.log(j[0]?.pid||\"\")}catch{console.log(\"\")} })')" C-m
tmux send-keys -t "$SESSION":0.1 'if [ -n "$PID" ]; then kill -9 "$PID" || true; fi' C-m
sleep 1
tmux send-keys -t "$SESSION":0.1 "./cli/agent-editor plugin call-core echo '{\"jsonrpc\":\"2.0\",\"id\":\"2\",\"method\":\"fs.read\",\"params\":{\"path\":\"docs/README.md\"}}'" C-m

# Pane 2: CLI tail of plugin-prefixed log lines
tmux split-window -v -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.2 './cli/agent-editor plugin events tail --file .sidecar.log --follow' C-m

tmux select-layout -t "$SESSION":0 tiled
if [[ -z "${HEADLESS:-}" ]]; then
  tmux attach -t "$SESSION"
else
  # allow a short observation period in headless mode, then clean up
  sleep 3
  tmux kill-session -t "$SESSION" || true
fi

