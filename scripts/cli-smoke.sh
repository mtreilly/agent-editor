#!/usr/bin/env bash
set -euo pipefail

echo "[SMOKE] Checking JSON-RPC health (127.0.0.1:35678)"
node scripts/dev-check.mjs || { echo "Start desktop dev: pnpm dev"; exit 1; }

echo "[SMOKE] Building CLI"
pushd cli >/dev/null
GOFLAGS="" go build -o agent-editor ./cmd/agent-editor
CLI_BIN="$(pwd)/agent-editor"
popd >/dev/null

TMP_REPO="$(mktemp -d)"
echo "# Hello" > "$TMP_REPO/hello.md"
echo "This links to [[hello-2]]" >> "$TMP_REPO/hello.md"
echo "# Second" > "$TMP_REPO/hello-2.md"

echo "[SMOKE] repo add $TMP_REPO"
"$CLI_BIN" repo add "$TMP_REPO" -o json || true

echo "[SMOKE] repo scan $TMP_REPO"
SCAN_JSON=$("$CLI_BIN" repo scan "$TMP_REPO" -o json || true)
echo "$SCAN_JSON"
SCAN_ERRORS=$(echo "$SCAN_JSON" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{let j=JSON.parse(d);console.log(j.errors||0)}catch{console.log(999)}})")
DOCS_ADDED=$(echo "$SCAN_JSON" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{let j=JSON.parse(d);console.log(j.docs_added||0)}catch{console.log(0)}})")
if [ "${SCAN_ERRORS:-0}" -gt 0 ]; then
  echo "[SMOKE] FAIL: scan errors=$SCAN_ERRORS" >&2
  exit 1
fi
if [ "${DOCS_ADDED:-0}" -eq 0 ]; then
  echo "[SMOKE] WARN: docs_added=0 (continuing)" >&2
fi

echo "[SMOKE] search 'Hello'"
SEARCH_JSON=$("$CLI_BIN" doc search Hello -o json || true)
echo "$SEARCH_JSON" >/dev/null
echo "$SEARCH_JSON" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{JSON.parse(d);process.exit(0)}catch{process.exit(1)}})" || { echo "[SMOKE] FAIL: search did not return valid JSON" >&2; exit 1; }

echo "[SMOKE] FTS stats"
STATS=$("$CLI_BIN" fts stats -o json || true)
echo "$STATS"
COUNT=$(echo "$STATS" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{let j=JSON.parse(d);console.log(j.fts_count||0)}catch{console.log(0)}})" )
MISSING=$(echo "$STATS" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{let j=JSON.parse(d);console.log(j.fts_missing||0)}catch{console.log(999)}})" )
if [ "${MISSING:-0}" -gt 0 ]; then
  echo "[SMOKE] FAIL: fts_missing=$MISSING (expected 0)" >&2
  exit 1
fi

echo "[SMOKE] graph backlinks (should be empty for new docs)"
FIRST_ID=$("$CLI_BIN" doc search Hello -o json | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | head -n1)
if [[ -n "${FIRST_ID:-}" ]]; then
  "$CLI_BIN" graph backlinks "$FIRST_ID" -o json || true
  "$CLI_BIN" graph neighbors "$FIRST_ID" -o json || true
fi

echo "[SMOKE] Done. Temp repo: $TMP_REPO"
