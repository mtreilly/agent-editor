# Contributing to agent-editor

Thanks for your interest! This project aims for small, clear changes with strong docs.

## Getting started
- Read `docs/guides/CODEMAP.md` to orient.
- Use tmux scripts (`AGENTS.md` → Vibe + Tmux) for dev/test.
- Run `HEADLESS=1 pnpm tmux:e2e` and `pnpm tmux:smoke` before submitting.

## Conventions
- Conventional commits: `feat(scope): ...`, `fix(scope): ...`, `docs(scope): ...`.
- Keep PRs small; one logical change per PR.
- i18n: extract visible strings to `public/locales/*/*.json`.
- No barrel files; direct imports.
- Use `fd`, `ag`, `ast-grep` for search per `AGENTS.md`.

## Safety and privacy
- Don’t persist secrets; provider keys go to OS keychain when enabled.
- `redact()` masks sensitive tokens before storing AI traces.

## Asking questions
- Open a draft PR with context if you’re unsure; update `docs/OPEN_QUESTIONS.md` when relevant.
