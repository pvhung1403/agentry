import Foundation

/// Where one agent keeps its things, as paths relative to the home folder.
/// To support a new agent, add an entry to `AgentSpec.all` and a fixture test.
public struct AgentSpec: Sendable {
    public let name: String
    /// The agent is shown only when this path exists.
    let root: String
    /// CLI used to remove things the agent manages itself (e.g. Codex MCP servers).
    var cli: String? = nil
    /// Each child folder that holds a SKILL.md is a skill.
    var skillDirs: [String] = []
    /// Each *.md file is a subagent.
    var subagentDirs: [String] = []
    /// Each *.md / *.toml file is a slash command or saved prompt.
    var commandDirs: [String] = []
    /// Instruction files loaded into every session.
    var memoryFiles: [String] = []
    /// Each *.md file inside is a rule or memory file.
    var memoryDirs: [String] = []
    /// JSON files with a top-level "mcpServers" object.
    var mcpJSON: [String] = []
    /// TOML files with [mcp_servers.<name>] tables.
    var mcpTOML: [String] = []
    /// Each child folder is a plugin or extension.
    var pluginDirs: [String] = []
    /// JSON files that contain hook commands.
    var hookFiles: [String] = []
}

extension AgentSpec {
    static let claudeName = "Claude Code"

    public static let all: [AgentSpec] = [
        // Claude plugins, marketplaces, MCP servers (~/.claude.json) and project memory are in InventoryScanner+Claude.
        AgentSpec(name: claudeName, root: ".claude", cli: "claude",
                  skillDirs: [".claude/skills"], subagentDirs: [".claude/agents"], commandDirs: [".claude/commands"],
                  memoryFiles: [".claude/CLAUDE.md"], hookFiles: [".claude/settings.json"]),
        AgentSpec(name: "Codex", root: ".codex", cli: "codex",
                  skillDirs: [".codex/skills"], commandDirs: [".codex/prompts"],
                  memoryFiles: [".codex/AGENTS.md"], mcpTOML: [".codex/config.toml"], hookFiles: [".codex/hooks.json"]),
        AgentSpec(name: "Cursor", root: ".cursor",
                  skillDirs: [".cursor/skills"], subagentDirs: [".cursor/agents"], commandDirs: [".cursor/commands"],
                  mcpJSON: [".cursor/mcp.json"], pluginDirs: [".cursor/plugins/local"], hookFiles: [".cursor/hooks.json"]),
        AgentSpec(name: "Gemini CLI", root: ".gemini", cli: "gemini",
                  skillDirs: [".gemini/skills"], commandDirs: [".gemini/commands"],
                  memoryFiles: [".gemini/GEMINI.md"], mcpJSON: [".gemini/settings.json"],
                  pluginDirs: [".gemini/extensions"], hookFiles: [".gemini/settings.json"]),
        AgentSpec(name: "Antigravity", root: ".gemini/antigravity",
                  skillDirs: [".gemini/antigravity/skills"], mcpJSON: [".gemini/config/mcp_config.json"],
                  pluginDirs: [".gemini/config/plugins"], hookFiles: [".gemini/config/hooks.json"]),
        AgentSpec(name: "Copilot CLI", root: ".copilot",
                  skillDirs: [".copilot/skills"], subagentDirs: [".copilot/agents"],
                  memoryFiles: [".copilot/copilot-instructions.md"], mcpJSON: [".copilot/mcp-config.json"],
                  pluginDirs: [".copilot/installed-plugins"]),
        AgentSpec(name: "Windsurf", root: ".codeium/windsurf",
                  skillDirs: [".codeium/windsurf/skills"], memoryFiles: [".codeium/windsurf/memories/global_rules.md"],
                  mcpJSON: [".codeium/windsurf/mcp_config.json"]),
        // Shared folder read by several agents (and written by `npx skills`).
        AgentSpec(name: "Shared (~/.agents)", root: ".agents",
                  skillDirs: [".agents/skills"], memoryDirs: [".agents/rules"], pluginDirs: [".agents/plugins"]),
    ]
}
