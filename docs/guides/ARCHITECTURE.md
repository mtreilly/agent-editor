# Architecture — agent-editor

```mermaid
flowchart TD
  subgraph Desktop[Tauri 2 App]
    FE[React 19 + TanStack Start]
    IPC[Tauri IPC]
    CORE[Core Runtime (Rust)]
  end

  DB[(SQLite + FTS5)]
  SCN[Scanner (.gitignore-aware)]
  GR[Graph Service]
  AI[AI Providers]

  FE <---> IPC
  IPC <--> CORE
  CORE <--> DB
  CORE <--> SCN
  CORE <--> GR
  CORE <--> AI

  subgraph Sidecar
    RPC[JSON-RPC HTTP 127.0.0.1:35678]
  end
  RPC <--> CORE

  subgraph CLI
    GO[Go CLI]
  end
  GO --> RPC
```

Key:
- Desktop UI uses Tauri IPC; CLI uses JSON-RPC sidecar.
- SQLite stores all data; FTS external-content indices.
- Scanner maintains docs/versions/links; Graph provides neighbors/backlinks/path.
```
