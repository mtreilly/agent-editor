#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-e2e"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }

tmux new-session -d -s "$SESSION" -n e2e

# Pane 0: Web dev server
tmux send-keys -t "$SESSION":0.0 'pnpm dev:web' C-m

# Pane 1: Playwright tests
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'pnpm test:e2e' C-m

tmux select-layout -t "$SESSION":0 tiled
if [[ -n "${HEADLESS:-}" ]]; then
  echo "[tmux-e2e] HEADLESS set; not attaching to tmux session '$SESSION'" >&2
else
  tmux attach -t "$SESSION"
fi
