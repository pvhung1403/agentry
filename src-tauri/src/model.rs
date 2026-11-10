use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Skill,
    Plugin,
    Memory,
    Subagent,
    Command,
    Mcp,
    Hook,
    Marketplace,
    Leftover,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Skill => "skill",
            Kind::Plugin => "plugin",
            Kind::Memory => "memory",
            Kind::Subagent => "subagent",
            Kind::Command => "command",
            Kind::Mcp => "mcp",
            Kind::Hook => "hook",
            Kind::Marketplace => "marketplace",
            Kind::Leftover => "leftover",
        }
    }
}

/// How an item is removed. The agent's own CLI is preferred so its state files stay consistent.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Removal {
    Trash { path: PathBuf },
    /// argv[0] is a tool name (e.g. "claude"), resolved to a full path when run.
    Cli { argv: Vec<String>, cwd: Option<PathBuf> },
    /// Deletes one key from a JSON config file, after saving a backup.
    JsonKey { file: PathBuf, key_path: Vec<String> },
    /// Agentry can't do it safely; the user edits this file by hand.
    Manual { file: PathBuf },
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Toggle {
    pub enable: Vec<String>,
    pub disable: Vec<String>,
}

/// Facts about an item. The UI turns them into localized text.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "code", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Note {
    Version { value: String },
    Contents { skills: usize, agents: usize, commands: usize, hooks: usize },
    LinkTo { target: String },
    Hooks { count: usize, location: String },
    ProjectScope { project: String },
    ProjectMemory { count: usize },
    UnusedMarketplace,
    Leftover,
    RestartAfterClean,
    FolderRefs,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub agent: String,
    pub kind: Kind,
    pub name: String,
    pub summary: String,
    pub path: Option<PathBuf>,
    pub bytes: u64,
    /// Estimated tokens this item adds to the context (descriptions, always-loaded memory).
    pub tokens: u64,
    /// False when the tokens only load in one project, so they stay out of the per-session total.
    pub per_session: bool,
    /// None when the item has no on/off switch.
    pub enabled: Option<bool>,
    pub toggle: Option<Toggle>,
    pub removal: Removal,
    pub notes: Vec<Note>,
}

impl Item {
    pub fn new(agent: &str, kind: Kind, name: impl Into<String>, path: Option<PathBuf>, removal: Removal) -> Item {
        let name = name.into();
        let location = path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        Item {
            id: format!("{agent}|{}|{location}|{name}", kind.as_str()),
            agent: agent.to_string(),
            kind,
            name,
            summary: String::new(),
            path,
            bytes: 0,
            tokens: 0,
            per_session: true,
            enabled: None,
            toggle: None,
            removal,
            notes: Vec::new(),
        }
    }

    /// Makes the id unique when one file holds several items with the same name.
    pub fn tagged(mut self, tag: &str) -> Item {
        self.id.push('|');
        self.id.push_str(tag);
        self
    }
}
