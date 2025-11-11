# Packaging — Multi‑OS Build & Signing

This guide explains how to package the Tauri desktop app across macOS, Windows, and Linux, including CI matrix examples and signing/notarization notes.

## Prerequisites
- Node.js 18+, pnpm
- Rust toolchain (stable)
- Tauri prerequisites per OS
  - macOS: Xcode CLT; for signing, Developer ID Application cert + Apple ID for notarization
  - Windows: Visual Studio Build Tools; for signing, code signing certificate (PFX) + signtool
  - Linux: build-essential and platform libs; optional signing via GPG

## Local packaging
```bash
pnpm build            # Vite client
pnpm tauri build      # Package app
# Recommended tmux orchestrator
HEADLESS=1 pnpm tmux:tauri-build
```

Ensure `src-tauri/icons/icon.png` is 32‑bit RGBA (e.g., 512×512). See BUILD.md for troubleshooting.

## CI: GitHub Actions matrix (example)
```yaml
name: build
on:
  push:
    branches: [ main ]
  pull_request:

jobs:
  tauri:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with: { version: 9 }
      - uses: actions/setup-node@v4
        with:
          node-version: 18
          cache: pnpm
      - name: Install system deps (Linux)
        if: startsWith(matrix.os, 'ubuntu')
        run: sudo apt-get update && sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
      - name: Install deps
        run: pnpm install --frozen-lockfile
      - name: Build (tmux wrapper)
        env:
          HEADLESS: 1
        run: pnpm tmux:tauri-build
      - name: Archive artifacts
        uses: actions/upload-artifact@v4
        with:
          name: app-${{ matrix.os }}
          path: |
            src-tauri/target/release/bundle/**
```

## Codesigning & Notarization

macOS (Developer ID + Notarization):
- Create a keychain entry for your Developer ID Application certificate.
- Set env in CI: `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`.
- Configure Tauri signing in `tauri.conf.json` or via env (see Tauri docs).
- Notarization runs post-build; ensure artifacts are `.app`/`.dmg`.

Windows (signtool):
- Store PFX in CI secrets or use a secure certificate store.
- Export `CSC_LINK` (to PFX) and `CSC_KEY_PASSWORD` (if needed) and configure Tauri signing hooks.

Linux:
- Optional: sign AppImage or packages using GPG; publish signatures alongside artifacts.

## Vibe notifications in CI
- Optionally send Discord messages on start/progress/done using `pnpm vibe:*` wrappers.
- In CI, set `HEADLESS=1` or call `vibe discord message send --content "…"` directly.

## Tips
- Use `pnpm approve-builds` when toolchain prompts need approval (locally).
- Keep artifacts under `src-tauri/target/release/bundle/` and upload per‑OS.
- Verify that `AE_DB` is set appropriately or that dev DB files are not included in artifacts.

