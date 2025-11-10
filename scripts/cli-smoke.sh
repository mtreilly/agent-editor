#!/usr/bin/env bash
set -euo pipefail

echo "[SMOKE] Checking JSON-RPC health (127.0.0.1:35678)"
node scripts/dev-check.mjs || { echo "Start desktop dev: pnpm dev"; exit 1; }

echo "[SMOKE] Building CLI"
pushd cli >/dev/null
GOFLAGS="" go build -o agent-editor
CLI_BIN="$(pwd)/agent-editor"
popd >/dev/null

TMP_REPO="$(mktemp -d)"
echo "# Hello" > "$TMP_REPO/hello.md"
echo "This links to [[hello-2]]" >> "$TMP_REPO/hello.md"
echo "# Second" > "$TMP_REPO/hello-2.md"

echo "[SMOKE] repo add $TMP_REPO"
"$CLI_BIN" repo add "$TMP_REPO" -o json || true

echo "[SMOKE] repo scan $TMP_REPO"
"$CLI_BIN" repo scan "$TMP_REPO" -o json || true

echo "[SMOKE] search 'Hello'"
"$CLI_BIN" doc search Hello -o json || true

echo "[SMOKE] graph backlinks (should be empty for new docs)"
FIRST_ID=$("$CLI_BIN" doc search Hello -o json | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | head -n1)
if [[ -n "${FIRST_ID:-}" ]]; then
  "$CLI_BIN" graph backlinks "$FIRST_ID" -o json || true
  "$CLI_BIN" graph neighbors "$FIRST_ID" -o json || true
fi

echo "[SMOKE] Done. Temp repo: $TMP_REPO"

