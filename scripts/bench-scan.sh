#!/usr/bin/env bash
set -euo pipefail

REPO_PATH="${1:-}"
N_FILES="${2:-1000}"

if ! pgrep -f rpc_sidecar >/dev/null 2>&1; then
  echo "[bench-scan] Starting RPC sidecar..." >&2
  (cd src-tauri && cargo run --quiet --bin rpc_sidecar >/dev/null 2>&1 & echo $! > ../.sidecar.pid)
  sleep 1
fi

echo "[bench-scan] Building CLI..." >&2
(cd cli && go build -o agent-editor ./cmd/agent-editor)
CLI="./cli/agent-editor"

TEMP_CREATED=0
if [[ -z "${REPO_PATH}" ]]; then
  REPO_PATH="$(mktemp -d)"
  TEMP_CREATED=1
  echo "[bench-scan] Generating ${N_FILES} markdown files under ${REPO_PATH}" >&2
  i=1
  while [[ $i -le ${N_FILES} ]]; do
    fn="${REPO_PATH}/note-${i}.md"
    printf "# Note %d\n\nThis links to [[note-%d]] and [[note-%d|Alias]].\n" "$i" "$(( (i%10)+1 ))" "$(( (i%25)+1 ))" > "$fn"
    i=$((i+1))
  done
fi

echo "[bench-scan] Repo: ${REPO_PATH}" >&2
START_MS=$(node -e 'console.log(Date.now())')
${CLI} repo add "${REPO_PATH}" -o json >/dev/null || true
OUT=$(${CLI} repo scan "${REPO_PATH}" -o json || true)
END_MS=$(node -e 'console.log(Date.now())')
ELAPSED_MS=$((END_MS - START_MS))

DOCS_ADDED=$(echo "$OUT" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{let j=JSON.parse(d);console.log(j.docs_added||0)}catch{console.log(0)}})")
FILES_SCANNED=$(echo "$OUT" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{let j=JSON.parse(d);console.log(j.files_scanned||0)}catch{console.log(0)}})")

THROUGHPUT=$(node -e "const ms=${ELAPSED_MS}||1, n=${DOCS_ADDED}||0; console.log((n/(ms/1000)).toFixed(2))")

echo "{\"files_scanned\": ${FILES_SCANNED}, \"docs_added\": ${DOCS_ADDED}, \"elapsed_ms\": ${ELAPSED_MS}, \"docs_per_sec\": ${THROUGHPUT}}"

if [[ ${TEMP_CREATED} -eq 1 ]]; then
  echo "[bench-scan] Temp repo kept at ${REPO_PATH}" >&2
fi

