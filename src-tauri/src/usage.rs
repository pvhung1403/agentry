use crate::model::{Component, Item, Kind, Note};
use crate::specs::CLAUDE;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// When skills, subagents, MCP servers and plugins were last used, read from Claude Code's
/// session transcripts (~/.claude/projects/**/*.jsonl). Claude Code deletes old transcripts,
/// so "not used" means "not used since `since`".
#[derive(Default, Debug)]
pub struct Usage {
    /// "skill:<name>", "agent:<name>", "mcp:<server>", "plugin:<plugin>" -> newest ISO-8601 timestamp.
    pub last: HashMap<String, String>,
    /// Plugin name -> the different parts of it that were used ("<plugin>:<skill>", "mcp:<server>").
    pub plugin_parts: HashMap<String, HashSet<String>>,
    /// Oldest timestamp in the history.
    pub since: Option<String>,
}

impl Usage {
    pub fn read(projects: &Path) -> Usage {
        let mut usage = Usage::default();
        for file in jsonl_files(projects) {
            let Ok(text) = fs::read_to_string(&file) else { continue };
            // Transcripts are append-only, so the first timestamp is the file's oldest.
            if let Some(ts) = text.lines().find_map(timestamp) {
                usage.note_start(&ts);
            }
            for line in text.lines() {
                // Cheap filter before parsing: only these lines can record a use.
                if !line.contains("\"tool_use\"") && !line.contains("<command-name>") {
                    continue;
                }
                let Ok(entry) = serde_json::from_str::<Value>(line) else { continue };
                let Some(ts) = entry.get("timestamp").and_then(Value::as_str) else { continue };
                match entry.pointer("/message/content") {
                    Some(Value::String(text)) => usage.commands(text, ts),
                    Some(Value::Array(blocks)) => blocks.iter().for_each(|block| usage.block(block, ts)),
                    _ => {}
                }
            }
        }
        usage
    }

    /// Fills `tracked`, `last_used` and the plugin usage note on Claude Code items.
    pub fn apply(&self, items: &mut [Item]) {
        for item in items.iter_mut().filter(|i| i.agent == CLAUDE) {
            let key = match item.kind {
                // Claude Code runs slash commands as skills, so both share one namespace.
                Kind::Skill | Kind::Command => format!("skill:{}", item.name),
                Kind::Subagent => format!("agent:{}", item.name),
                Kind::Mcp => format!("mcp:{}", tool_name(&item.name)),
                Kind::Plugin => format!("plugin:{}", plugin_name(&item.name)),
                _ => continue,
            };
            item.tracked = true;
            item.last_used = self.last.get(&key).cloned();
            if item.kind == Kind::Plugin {
                let used = self.plugin_parts.get(plugin_name(&item.name)).map_or(0, HashSet::len);
                item.notes.push(Note::Usage { used });
                let has_hooks = item.notes.iter().any(|n| matches!(n, Note::Contents { hooks, .. } if *hooks > 0));
                if item.last_used.is_none() && has_hooks {
                    item.notes.push(Note::RunsViaHooks);
                }
            }
        }
    }

    /// When one part of a plugin was last used. Claude Code names it "<plugin>:<part>",
    /// and a plugin's MCP server "plugin_<plugin>_<server>".
    pub fn part_last_used(&self, plugin: &str, kind: Kind, name: &str) -> Option<String> {
        let key = match kind {
            Kind::Skill | Kind::Command => format!("skill:{plugin}:{name}"),
            Kind::Subagent => format!("agent:{plugin}:{name}"),
            Kind::Mcp => format!("mcp:plugin_{plugin}_{}", tool_name(name)),
            _ => return None,
        };
        self.last.get(&key).cloned()
    }

    /// The parts of a plugin, most expensive first, with when each was last used.
    pub fn components(&self, plugin: &Item, parts: Vec<Item>) -> Vec<Component> {
        let name = plugin_name(&plugin.name);
        let mut list: Vec<Component> = parts
            .into_iter()
            .map(|part| Component {
                last_used: if plugin.tracked { self.part_last_used(name, part.kind, &part.name) } else { None },
                tracked: plugin.tracked,
                kind: part.kind,
                name: part.name,
                summary: part.summary,
                tokens: part.tokens,
            })
            .collect();
        list.sort_by(|a, b| b.tokens.cmp(&a.tokens).then_with(|| a.name.cmp(&b.name)));
        list
    }

    fn block(&mut self, block: &Value, ts: &str) {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_use") => {
                let name = block.get("name").and_then(Value::as_str).unwrap_or_default();
                let field = |key: &str| block.get("input").and_then(|i| i.get(key)).and_then(Value::as_str);
                match name {
                    "Skill" => {
                        if let Some(skill) = field("skill") {
                            self.used("skill", skill, ts);
                        }
                    }
                    "Agent" | "Task" => {
                        if let Some(agent) = field("subagent_type") {
                            self.used("agent", agent, ts);
                        }
                    }
                    _ => {
                        if let Some(rest) = name.strip_prefix("mcp__") {
                            self.mcp(rest, ts);
                        }
                    }
                }
            }
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    self.commands(text, ts);
                }
            }
            _ => {}
        }
    }

    /// Slash commands the user typed: `<command-name>/plugin:name</command-name>`.
    fn commands(&mut self, text: &str, ts: &str) {
        const OPEN: &str = "<command-name>";
        let mut rest = text;
        while let Some(start) = rest.find(OPEN) {
            rest = &rest[start + OPEN.len()..];
            let Some(end) = rest.find("</command-name>") else { break };
            let name = rest[..end].trim().trim_start_matches('/');
            if !name.is_empty() {
                self.used("skill", name, ts);
            }
            rest = &rest[end..];
        }
    }

    /// `name` may be "<plugin>:<part>"; the plugin then counts as used too.
    fn used(&mut self, kind: &str, name: &str, ts: &str) {
        self.touch(format!("{kind}:{name}"), ts);
        if let Some((plugin, _)) = name.split_once(':') {
            self.touch(format!("plugin:{plugin}"), ts);
            self.plugin_parts.entry(plugin.to_string()).or_default().insert(name.to_string());
        }
    }

    /// MCP tools are named `mcp__<server>__<tool>`; plugin servers are `plugin_<plugin>_<server>`.
    fn mcp(&mut self, rest: &str, ts: &str) {
        let server = rest.split("__").next().unwrap_or(rest);
        self.touch(format!("mcp:{server}"), ts);
        // shortcut: a plugin name that contains "_" is split at the wrong place; rare in practice.
        if let Some((plugin, _)) = server.strip_prefix("plugin_").and_then(|s| s.split_once('_')) {
            self.touch(format!("plugin:{plugin}"), ts);
            self.plugin_parts.entry(plugin.to_string()).or_default().insert(format!("mcp:{server}"));
        }
    }

    fn touch(&mut self, key: String, ts: &str) {
        let newest = self.last.entry(key).or_default();
        // ISO-8601 UTC timestamps sort as strings.
        if ts > newest.as_str() {
            *newest = ts.to_string();
        }
    }

    fn note_start(&mut self, ts: &str) {
        if self.since.as_deref().map_or(true, |since| ts < since) {
            self.since = Some(ts.to_string());
        }
    }
}

/// "ecc@ecc" -> "ecc": the name Claude Code uses in "ecc:skill".
pub fn plugin_name(id: &str) -> &str {
    id.split('@').next().unwrap_or(id)
}

/// Claude Code builds MCP tool names from the server name with everything but [A-Za-z0-9_-] replaced by "_".
fn tool_name(server: &str) -> String {
    server.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect()
}

fn timestamp(line: &str) -> Option<String> {
    if !line.contains("\"timestamp\"") {
        return None;
    }
    serde_json::from_str::<Value>(line).ok()?.get("timestamp")?.as_str().map(str::to_string)
}

fn jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return out };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            out.extend(jsonl_files(&path));
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            out.push(path);
        }
    }
    out
}
