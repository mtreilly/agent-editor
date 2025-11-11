#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-vibe"
MSG="${1:-}";
HEADLESS="${HEADLESS:-}"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }
tmux new-session -d -s "$SESSION" -n notify
tmux send-keys -t "$SESSION":0.0 "bash scripts/vibe-notify.sh \"$MSG\"" C-m

if [[ -n "$HEADLESS" ]]; then
  # Allow command to run, then clean up session without attaching
  sleep 1
  tmux kill-session -t "$SESSION" || true
else
  tmux attach -t "$SESSION"
fi
