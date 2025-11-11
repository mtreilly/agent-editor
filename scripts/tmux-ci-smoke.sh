#!/usr/bin/env bash
set -euo pipefail

SESSION="ae-ci-smoke"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required. Install tmux and re-run." >&2
  exit 1
fi

tmux has-session -t "$SESSION" 2>/dev/null && { tmux kill-session -t "$SESSION" || true; }
tmux new-session -d -s "$SESSION" -n ci

# Pane 0: Sidecar + CLI smoke (within tmux-smoke script)
tmux send-keys -t "$SESSION":0.0 'HEADLESS=1 pnpm tmux:smoke' C-m

# Pane 1: E2E (web stubs)
tmux split-window -h -t "$SESSION":0.0
tmux send-keys -t "$SESSION":0.1 'HEADLESS=1 pnpm tmux:e2e' C-m

tmux select-layout -t "$SESSION":0 tiled
if [[ -n "${HEADLESS:-}" ]]; then
  echo "[tmux-ci-smoke] HEADLESS set; not attaching to tmux session '$SESSION'" >&2
else
  tmux attach -t "$SESSION"
fi

