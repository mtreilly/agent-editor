#!/usr/bin/env bash
set -euo pipefail

MSG="${1:-}"
CHANNEL="${VIBE_CHANNEL:-}"

if [[ -z "$MSG" ]]; then
  echo "Usage: scripts/vibe-notify.sh \"message text\"" >&2
  exit 2
fi

if command -v vibe >/dev/null 2>&1; then
  if [[ -n "$CHANNEL" ]]; then
    vibe discord send --channel "$CHANNEL" --message "$MSG" || true
  else
    vibe discord send --message "$MSG" || true
  fi
else
  echo "[vibe-notify] vibe CLI not found. Message: $MSG" >&2
fi

