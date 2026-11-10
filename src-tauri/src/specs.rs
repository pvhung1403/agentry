/// Where one agent keeps its things, as "/"-separated paths relative to the home folder.
/// The same layout is used on macOS, Windows (%USERPROFILE%) and Linux.
/// To support a new agent, add an entry to `ALL` and a fixture test in tests.rs.
pub struct AgentSpec {
    pub name: &'static str,
    /// The agent is shown only when this path exists.
    pub root: &'static str,
    /// CLI used to remove things the agent manages itself (e.g. Codex MCP servers).
    pub cli: Option<&'static str>,
    /// Each child folder that holds a SKILL.md is a skill.
    pub skill_dirs: &'static [&'static str],
    /// Each *.md file is a subagent.
    pub subagent_dirs: &'static [&'static str],
    /// Each *.md / *.toml file is a slash command or saved prompt.
    pub command_dirs: &'static [&'static str],
    /// Instruction files loaded into every session.
    pub memory_files: &'static [&'static str],
    /// Each *.md file inside is a rule or memory file.
    pub memory_dirs: &'static [&'static str],
    /// JSON files with a top-level "mcpServers" object.
    pub mcp_json: &'static [&'static str],
    /// TOML files with [mcp_servers.<name>] tables.
    pub mcp_toml: &'static [&'static str],
    /// Each child folder is a plugin or extension.
    pub plugin_dirs: &'static [&'static str],
    /// JSON files that contain hook commands.
    pub hook_files: &'static [&'static str],
}

pub const CLAUDE: &str = "Claude Code";

const EMPTY: AgentSpec = AgentSpec {
    name: "",
    root: "",
    cli: None,
    skill_dirs: &[],
    subagent_dirs: &[],
    command_dirs: &[],
    memory_files: &[],
    memory_dirs: &[],
    mcp_json: &[],
    mcp_toml: &[],
    plugin_dirs: &[],
    hook_files: &[],
};

pub const ALL: &[AgentSpec] = &[
    // Claude plugins, marketplaces, MCP servers (~/.claude.json) and project memory are in claude.rs.
    AgentSpec {
        name: CLAUDE,
        root: ".claude",
        cli: Some("claude"),
        skill_dirs: &[".claude/skills"],
        subagent_dirs: &[".claude/agents"],
        command_dirs: &[".claude/commands"],
        memory_files: &[".claude/CLAUDE.md"],
        hook_files: &[".claude/settings.json"],
        ..EMPTY
    },
    AgentSpec {
        name: "Codex",
        root: ".codex",
        cli: Some("codex"),
        skill_dirs: &[".codex/skills"],
        command_dirs: &[".codex/prompts"],
        memory_files: &[".codex/AGENTS.md"],
        mcp_toml: &[".codex/config.toml"],
        hook_files: &[".codex/hooks.json"],
        ..EMPTY
    },
    AgentSpec {
        name: "Cursor",
        root: ".cursor",
        skill_dirs: &[".cursor/skills"],
        subagent_dirs: &[".cursor/agents"],
        command_dirs: &[".cursor/commands"],
        mcp_json: &[".cursor/mcp.json"],
        plugin_dirs: &[".cursor/plugins/local"],
        hook_files: &[".cursor/hooks.json"],
        ..EMPTY
    },
    AgentSpec {
        name: "Gemini CLI",
        root: ".gemini",
        cli: Some("gemini"),
        skill_dirs: &[".gemini/skills"],
        command_dirs: &[".gemini/commands"],
        memory_files: &[".gemini/GEMINI.md"],
        mcp_json: &[".gemini/settings.json"],
        plugin_dirs: &[".gemini/extensions"],
        hook_files: &[".gemini/settings.json"],
        ..EMPTY
    },
    AgentSpec {
        name: "Antigravity",
        root: ".gemini/antigravity",
        skill_dirs: &[".gemini/antigravity/skills"],
        mcp_json: &[".gemini/config/mcp_config.json"],
        plugin_dirs: &[".gemini/config/plugins"],
        hook_files: &[".gemini/config/hooks.json"],
        ..EMPTY
    },
    AgentSpec {
        name: "Copilot CLI",
        root: ".copilot",
        skill_dirs: &[".copilot/skills"],
        subagent_dirs: &[".copilot/agents"],
        memory_files: &[".copilot/copilot-instructions.md"],
        mcp_json: &[".copilot/mcp-config.json"],
        plugin_dirs: &[".copilot/installed-plugins"],
        ..EMPTY
    },
    AgentSpec {
        name: "Windsurf",
        root: ".codeium/windsurf",
        skill_dirs: &[".codeium/windsurf/skills"],
        memory_files: &[".codeium/windsurf/memories/global_rules.md"],
        mcp_json: &[".codeium/windsurf/mcp_config.json"],
        ..EMPTY
    },
    // Shared folder read by several agents (and written by `npx skills`).
    AgentSpec {
        name: "Shared (~/.agents)",
        root: ".agents",
        skill_dirs: &[".agents/skills"],
        memory_dirs: &[".agents/rules"],
        plugin_dirs: &[".agents/plugins"],
        ..EMPTY
    },
];
