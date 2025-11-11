# AGENTS.md — Agent Editor Project

## Purpose
- Build a production-ready code editor optimized for AI agent workflows
- Agent-friendly: consistent APIs, structured outputs, excellent error handling
- Inspired by patterns from `../agent-discord`, `~/vibe-engineering`, and modern editor best practices

## Scope
- Code editing interface with AI agent integration
- File system operations and project management
- Command palette and keyboard navigation
- Extension system for agent capabilities
- Configuration management
- Integration-ready for agent workflows

## Quick Start (Local Dev)
- Prerequisites: Node.js 18+, pnpm
- Verify tools:
  ```bash
  node --version
  pnpm --version
  which ag fd ast-grep  # Code search tools
  ```
- Set up environment:
  ```bash
  pnpm install
  pnpm dev
  ```

## Project Structure
```
agent-editor/
├── AGENTS.md              # This file
├── CLAUDE.md              # Project-specific instructions
└── docs/
    ├── design/            # Design principles and patterns
    ├── plans/             # Project plans and roadmaps
    ├── guides/            # How-to guides
    ├── manual/            # API reference and manuals
    └── progress/          # Status updates and tracking
```

## Code Search & Discovery

### FORBIDDEN TOOLS
- NEVER use: `find`, `grep`, `ls -R`, `cat` (for searching)
- Exception: Only if user explicitly requests it

### Tool Selection Matrix (MANDATORY)

| Task | ONLY Use | Example |
|------|----------|---------|
| Function/class defs | `ast-grep` | `ast-grep --lang tsx -p 'const $VAR = useState($$$)'` |
| Import statements | `ast-grep` | `ast-grep --lang tsx -p 'import { $$$ } from "$MODULE"'` |
| React/custom hooks | `ast-grep` | `ast-grep --lang tsx -p 'use$HOOK($$$)'` |
| File discovery | `fd` | `fd -e ts -e tsx` |
| Directory structure | `fd` + `tree` | `fd -t d \| tree --fromfile -L 2` |
| Content search | `ag` | `fd -e ts \| ag --file-list - 'pattern'` |

**Tip**: `$VAR` = single capture, `$$$` = variadic matches

### Recipes
```bash
fd -e ts -e tsx src | ag --file-list - 'useState|useEffect'  # React hooks
fd -e ts -e tsx | ag --file-list - 'fetch|axios'             # API calls
fd -HI -t f -E .git | ag --file-list - 'TODO|FIXME'          # Find TODOs
fd -HI -t f -E .git | ag --file-list - 'AKIA|api.?key'       # Secrets scan
fd -t d -E .git -E node_modules | tree --fromfile -L 2       # Repo structure
fd -e ts -e tsx | ag --file-list - 'export.*function'        # Exported functions
```

## Configuration Management

### Config File (config.yaml)
```yaml
editor:
  theme: dark
  font_size: 14
  tab_size: 2
  language: en

agent:
  api_endpoint: ${AGENT_API_ENDPOINT}
  timeout: 30s
  retries: 3

logging:
  level: info
  format: json
  output: artifacts/logs/editor.log
```

### Environment Variables
- `AGENT_API_ENDPOINT` — Agent API endpoint URL
- `LOG_LEVEL` — Logging level (debug, info, warn, error)
- `NODE_ENV` — Environment (development, production, test)

## Development Workflow

### Code Organization
- Feature-based organization (not type-based)
- Features: `{feature}/components/`, `{feature}/hooks/`, `{feature}/utils/`
- Shared code in root-level directories
- Clear separation: UI components, business logic, API layer

### Naming Conventions
- No `new*` prefixes for constructors/factories or command builders. Use canonical names.
  - CLI commands: `repoCmd`, `docCmd`, `pluginCmd`, `rootCmd` (not `newRepoCmd`, etc.).
  - Keep names stable and clear; no migration concerns apply (greenfield).

### Tech Stack
**Frontend (React/TypeScript):**
- UI: shadcn/ui + Tailwind CSS v4
- Routing: React Router v7
- State: Zustand (auth: React Context)
- Forms: react-hook-form
- Build: Biome, pnpm, Vite + SWC, day.js
- **FORBIDDEN**: Chakra UI, barrel files, MobX, ESLint/Prettier, npm/yarn, moment.js

### Testing
- Unit tests: `*.test.ts(x)` files alongside implementation
- Integration tests: `tests/integration/`
- E2E tests: Playwright (test at 320px, 768px, 1024px, 1440px)
- Coverage target: >80% for core packages
- Test checklist: keyboard nav, light/dark mode, translations

### Building
```bash
pnpm install                      # Install dependencies
pnpm dev                          # Start dev server
pnpm build                        # Build for production
pnpm test                         # Run all tests
pnpm test:unit                    # Run unit tests
pnpm test:e2e                     # Run E2E tests
pnpm lint                         # Run linting
pnpm format                       # Format code
```

### Debugging
1. Read full error + stack trace
2. `git diff` to see recent changes
3. Add debug logging to trace execution
4. Test incrementally after fixes
5. Use browser DevTools for frontend issues
6. Document fix in commit message

## Internationalization (i18n)
- Extract ALL visible text to translation files
- NO hardcoded strings in components
- Structure: `/public/locales/{lang}/{namespace}.json`
- Use i18next with React bindings
- Support multiple namespaces per feature

## Accessibility
- Full keyboard navigation
- ARIA labels on all interactive elements
- WCAG AA contrast requirements
- Screen reader testing
- Focus management for modals and dialogs

## Mobile-First Design
- Design for 320px minimum width
- Test at breakpoints: 375px, 768px, 1024px, 1440px
- Touch-friendly targets (44x44px minimum)
- Responsive layouts with Tailwind

## Git Workflow (MANDATORY)
- Commit early and often
- Use conventional format: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`
- Scope commits by feature: `feat(editor):`, `fix(commands):`, `docs(guides):`
- Commit BEFORE major refactors or risky changes
- Use `git status`, `git diff`, `git log` continuously
- Keep commits atomic and focused on single changes

### Debugging with Git
- When errors occur: `git diff` (see changes), `git log` (trace history)
- Use `git bisect` for regression hunting
- Prefer frequent small commits over monolithic ones

## Docs Organization
- `docs/plans/` — project and architecture plans
- `docs/guides/` — how-to guides (usage, integration, deployment)
- `docs/manual/` — API reference and detailed manuals
- `docs/design/` — design principles and patterns
- `docs/progress/` — status updates and phase tracking
- Root files: `README.md` (overview), `AGENTS.md` (this guide), `CLAUDE.md` (AI instructions)

## Design Standards (Critical)
- Follow design principles in `docs/design/`
- All features should have: proper error handling, keyboard shortcuts, i18n support
- When deviating, document rationale in `docs/OPEN_QUESTIONS.md`
- UI components must be accessible and responsive

## Open Questions (Living Log)
- Maintain `docs/OPEN_QUESTIONS.md` actively
- When encountering uncertainty, blocked decisions, or design tradeoffs, add an entry
- Keep entries concise with context, options, and next experiments
- Close items by linking to resolving commits/PRs/docs
- Treat this as handoff hygiene: leave open threads visible for the next agent

## Next Actions for Agents
- Review `docs/OPEN_QUESTIONS.md` for active discussions
- Check `docs/progress/STATUS.md` for current work
- Start with basic editor interface (most foundational)
- Write comprehensive tests alongside implementation
- Document all public APIs with JSDoc comments
- Add i18n for all user-facing text

## Testing & Validation
- All exported functions must have tests
- Use test-driven development for complex features
- Mock external dependencies (APIs, file system)
- E2E tests for critical user flows
- Accessibility testing with screen readers
- Performance benchmarks for large files

## Common Tasks

### Start Development
```bash
pnpm install
pnpm dev
```

### Run Tests
```bash
pnpm test                         # All tests
pnpm test:unit                    # Unit tests only
pnpm test:e2e                     # E2E tests only
pnpm test:watch                   # Watch mode
```

### Format & Lint
```bash
pnpm lint                         # Check linting
pnpm format                       # Format code
pnpm check                        # Type checking
```

### Build & Deploy
```bash
pnpm build                        # Production build
pnpm preview                      # Preview production build
```

## Borrowed Patterns (Do This)
- Small, atomic commits with conventional prefixes
- Feature-based organization
- Context propagation throughout
- Structured logging with levels
- Test-driven development
- Error boundaries for React components
- Configuration precedence: props > env > config > defaults
- Direct imports (no barrel files)

## Anti-Patterns (Don't Do This)
- Global state without justification
- Hardcoded strings (use i18n)
- Blocking operations without loading states
- Silent failures or swallowed errors
- Inconsistent error handling
- Missing JSDoc comments
- Untested code paths
- Barrel files (index.ts exports)
- Type assertions without necessity
- `new*` prefixes for functions/classes (e.g., `newPluginCmd`) — use canonical names (`pluginCmd`).

## Browser Automation
- Use fangagent for all browser automation tasks
- Health check: `fangagent doctor`
- Start server: `fangagent serve` (separate terminal)
- Verify UI changes with fangagent

## Terminal Orchestration (tmux — Mandatory)
- Always run ALL pnpm/cargo/go commands via tmux scripts. Do not run these directly.
- Use separate panes/windows per concern for observability:
  - Pane A: JSON‑RPC sidecar (`cargo run --manifest-path src-tauri/Cargo.toml --bin rpc_sidecar`)
  - Pane B: Web dev server (`pnpm dev:web`) or Tauri (`pnpm dev`)
  - Pane C: CLI smoke/benches (`bash scripts/cli-smoke.sh`, `scripts/bench-fts.sh`)
  - Pane D: Logs (`tail -f ./.sidecar.log`)
- Use provided scripts:
  - `pnpm tmux:dev` — sets up a 2x2 tmux layout with the above panes
  - `pnpm tmux:smoke` — sidecar + CLI smoke
  - `pnpm tmux:bench` — sidecar + FTS and scan benchmarks in dedicated panes
  - `pnpm tmux:e2e` — web dev server + Playwright tests
  - `pnpm tmux:bootstrap` — installs deps (pnpm), runs cargo check and CLI build in panes
  - `pnpm vibe:start` / `pnpm vibe:progress` / `pnpm vibe:done` — send Discord notifications via vibe CLI in a tmux session (requires `vibe` installed; otherwise logs locally)

## Notifications (Mandatory)
- Always send a Discord message via vibe CLI when you start, periodically during work, and when you finish:
  - Start: `pnpm vibe:start`
  - Progress: `pnpm vibe:progress`
  - Done: `pnpm vibe:done`
- These commands run inside tmux and invoke `vibe discord send`. Set `VIBE_CHANNEL` to target a specific Discord channel. If `vibe` is not installed, the script logs locally.
- If tmux is unavailable, install it or run an equivalent multi‑pane terminal.

## Multi-Agent Collaboration
- Git worktrees for parallel branches
- Independent dev servers on different ports
- Document completed work before handoff
- Keep `docs/OPEN_QUESTIONS.md` updated

## Context Management (Large Projects)
- Use `/compact` to summarize
- Split plans into subdirectories
- Move completed tasks to archive files
- Use `docs/process/{Phase-name}.md` for tracking work
- Extract critical instructions to local `AGENTS.md`

## Reference Resources
- React docs: https://react.dev
- TypeScript: https://www.typescriptlang.org/docs
- shadcn/ui: https://ui.shadcn.com
- Tailwind CSS: https://tailwindcss.com/docs
- WCAG Guidelines: https://www.w3.org/WAI/WCAG21/quickref

## Quick Reference

### Environment Setup
```bash
export NODE_ENV="development"
export LOG_LEVEL="debug"
export AGENT_API_ENDPOINT="http://localhost:3000"
```

### File Operations
```bash
fd -e ts -e tsx src                                    # Find TypeScript files
ag 'pattern' src                                       # Search in source
ast-grep --lang tsx -p 'useState($$$)' src             # Find useState calls
```

### Testing Shortcuts
```bash
pnpm test -- src/features/editor                       # Test specific feature
pnpm test:e2e -- --headed                              # E2E with browser visible
pnpm test -- --coverage                                # With coverage report
```
