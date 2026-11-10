# Loadout

A native macOS app to see and clean up everything your AI coding agents load: skills, plugins, memory, MCP servers, subagents, commands and hooks, across Claude Code, Codex, Cursor, Gemini CLI and more.

Installing skills and plugins is one command. Removing them is a hunt through hidden folders. Over time they pile up, and they cost more than disk space: every enabled skill's description is loaded into **every session's context**, and every hook runs on **every tool call**. Loadout shows all of it in one window, with what each item costs, and removes it in one click.

## What it shows

- **One table for every agent** — name, agent, kind, size on disk, estimated tokens per session, on/off state. Sort, search, multi-select.
- **Token cost** — skill/agent/command descriptions and always-loaded memory files (including `@imports`), so you can see which plugin eats your context.
- **Leftovers** — old plugin versions still in the cache, ready to reclaim.
- **Unused marketplaces** — plugin repos no installed plugin comes from.
- **Inspector** — where an item lives, what removing it will do, and a preview of the file.

## Supported agents

| Agent | Skills | Plugins | Memory | MCP | Subagents / Commands | Hooks |
|---|---|---|---|---|---|---|
| Claude Code | `~/.claude/skills` | installed plugins, cache, marketplaces | `CLAUDE.md`, project auto-memory | `~/.claude.json` (user + local) | `~/.claude/agents`, `commands` | `settings.json` |
| Codex | `~/.codex/skills` | — | `AGENTS.md` | `config.toml` | `prompts` | `hooks.json` |
| Cursor | `~/.cursor/skills` | `plugins/local` | — | `mcp.json` | `agents`, `commands` | `hooks.json` |
| Gemini CLI | `~/.gemini/skills` | `extensions` | `GEMINI.md` | `settings.json` | `commands` | `settings.json` |
| Antigravity | `~/.gemini/antigravity/skills` | `config/plugins` | — | `config/mcp_config.json` | — | `config/hooks.json` |
| Copilot CLI | `~/.copilot/skills` | `installed-plugins` | `copilot-instructions.md` | `mcp-config.json` | `agents` | — |
| Windsurf | `~/.codeium/windsurf/skills` | — | `memories/global_rules.md` | `mcp_config.json` | — | — |
| Shared | `~/.agents/skills` | `~/.agents/plugins` | `~/.agents/rules` | — | — | — |

Agents that aren't installed are hidden.

## Safety

- **Read-only until you click Remove**, and every removal asks first and says exactly what it will do.
- **Files go to the Trash**, never deleted outright.
- **Agent CLIs first**: Claude plugins, marketplaces and MCP servers are removed with `claude plugin uninstall`, `claude plugin marketplace remove` and `claude mcp remove`, so Claude's own state stays consistent.
- **Config edits are backed up** to `~/Library/Application Support/Loadout/Backups/` before a JSON key is removed.
- **No network, no telemetry.** Loadout only reads files in your home folder.

## Install

Requires macOS 14 or later.

```sh
git clone https://github.com/<you>/loadout.git
cd loadout
scripts/build-app.sh
open build/Loadout.app
```

For development: `swift run Loadout`, or open `Package.swift` in Xcode.

## Contributing

Supporting a new agent is usually one entry in [`AgentSpec.all`](Sources/LoadoutCore/AgentSpec.swift): list where it keeps skills, memory, MCP config and hooks. Then add a fixture to [the tests](Tests/LoadoutCoreTests/LoadoutCoreTests.swift) — tests build a fake home folder, so they never touch your real config.

When an agent changes its file layout, a fixture that reproduces the new layout is the most useful bug report.

```sh
swift test
```

## License

MIT
