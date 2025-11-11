# Vibe Notifications — Discord Messaging

This project requires Discord notifications at task start, during progress, and on completion.
Notifications keep multi-agent work visible and auditable.

## Quick Start
- Default channel is configured in your local `~/.config/vibe/discord.yaml` (or repo config).
- Send messages with the `vibe` CLI or via the tmux wrappers.

Commands
- Start: `pnpm vibe:start`
- Progress: `pnpm vibe:progress`
- Done: `pnpm vibe:done`

If tmux is unavailable (CI/headless), call the CLI directly:
- `vibe discord message send --content "agent-editor: Starting [task]"`
- `vibe discord message send --content "agent-editor: [task] progress…"`
- `vibe discord message send --content "agent-editor: Completed [task]"`

## Configuration
- Default channel ID: set `default_channel_id` in `discord.yaml`.
- Override channel: set `VIBE_CHANNEL` env var or pass `--channel <id>`.
- Profiles/environments: use `--profile` or `--env` as defined in `discord.yaml`.

## Examples
- Notify at phase start:
  - `vibe discord message send --content "agent-editor: M3 docs polish started on main."`
- Progress within a task:
  - `vibe discord message send --content "agent-editor: Updating AGENTS.md and guides (2/3)."`
- Include commit information:
  - `vibe discord message send --content "agent-editor: docs polish committed: 1a2b3c4 - docs(agents): clarify vibe notifications."`

## Troubleshooting
- If `pnpm vibe:*` fails with a terminal error, run with `HEADLESS=1` or use direct `vibe discord message send`.
- If `unknown flag: --message`, your `vibe` CLI expects `--content` (use the direct commands above).
- Validate config: `vibe discord config --output yaml`.
- Check permissions: ensure bot token or webhook is valid for the target channel.

