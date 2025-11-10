# Build & Package — agent-editor

## Prerequisites
- Node.js 18+
- pnpm
- Rust toolchain (rustup, stable)
- Platform build tools
  - macOS: Xcode Command Line Tools
  - Windows: Visual Studio Build Tools
  - Linux: build-essential, libgtk, etc. per Tauri docs

## Dev
```bash
pnpm install
pnpm dev         # launches Vite + Tauri dev (desktop)
```

## Dev Health Check
```bash
pnpm dev:check   # attempts JSON-RPC call to /rpc repos_list on 127.0.0.1:35678
```

## Build
```bash
pnpm build           # Vite client
pnpm tauri build     # Packages desktop app (after Vite build)
```

Notes:
- Ensure a valid RGBA PNG icon at `src-tauri/icons/icon.png` (e.g., 512×512, 32‑bit RGBA). If missing or not RGBA, Tauri build will fail.
- Approve SWC/esbuild build scripts if prompted: `pnpm approve-builds`.

Dev/Test defaults:
- The desktop app uses `AE_DB` if set, otherwise `.dev/agent-editor.db` to simplify local runs and CI.
- A minimal RGBA icon is included to unblock builds; replace with your branded 512×512 icon before packaging release artifacts.

## Troubleshooting
- Icon error: Convert to 32-bit RGBA (e.g., `magick input.png -define png:color-type=6 icon.png`).
- Port conflicts: JSON-RPC runs on 127.0.0.1:35678. Adjust via command arg or config if needed.
- Rust crates: run `cargo clean -p agent-editor` if schema or icons changed and rebuild.
