#!/usr/bin/env bash
set -euo pipefail

if ! pgrep -f rpc_sidecar >/dev/null 2>&1; then
  echo "Starting RPC sidecar..." >&2
  (cd src-tauri && cargo run --quiet --bin rpc_sidecar >/dev/null 2>&1 & echo $! > ../.sidecar.pid)
  sleep 1
fi

echo "Building CLI..." >&2
(cd cli && go build -o agent-editor ./cmd/agent-editor)

QUERY=${1:-Hello}
N=${2:-50}

./cli/agent-editor fts bench --query "$QUERY" --n "$N" -o json || true
