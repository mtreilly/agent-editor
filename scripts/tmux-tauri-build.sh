#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-tauri-build"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }

tmux new-session -d -s "$SESSION" -n build

# Pane 0: Frontend build
tmux send-keys -t "$SESSION":0.0 'pnpm build' C-m

# Pane 1: Tauri packaging
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'pnpm tauri build' C-m

# Pane 2: Logs (optional)
tmux split-window -v -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.2 'echo "[logs] attach to observe build output; panes 0/1 running"' C-m

tmux select-layout -t "$SESSION":0 tiled
if [[ -n "${HEADLESS:-}" ]]; then
  echo "[tmux-tauri-build] HEADLESS set; not attaching to tmux session '$SESSION'" >&2
else
  tmux attach -t "$SESSION"
fi

