use crate::frontmatter;
use crate::model::{Item, Kind, Note, Removal};
use crate::specs::{AgentSpec, ALL, CLAUDE};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Reads every agent's folders and builds the item list. Read-only: it never changes a file.
pub struct Scanner {
    pub home: PathBuf,
}

#[derive(Default)]
pub struct PluginInventory {
    pub tokens: u64,
    pub skills: usize,
    pub agents: usize,
    pub commands: usize,
    pub hooks: usize,
}

impl PluginInventory {
    pub fn notes(&self) -> Vec<Note> {
        if self.skills + self.agents + self.commands + self.hooks == 0 {
            return vec![];
        }
        vec![Note::Contents { skills: self.skills, agents: self.agents, commands: self.commands, hooks: self.hooks }]
    }
}

impl Scanner {
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }

    pub fn scan_all(&self) -> Vec<Item> {
        let mut items = Vec::new();
        for spec in ALL {
            if !self.rel(spec.root).exists() {
                continue;
            }
            items.extend(self.scan(spec));
            if spec.name == CLAUDE {
                items.extend(self.scan_claude_extras());
            }
        }
        items
    }

    /// Joins a "/"-separated relative path onto the home folder with native separators.
    pub fn rel(&self, relative: &str) -> PathBuf {
        join_rel(&self.home, relative)
    }

    fn scan(&self, spec: &AgentSpec) -> Vec<Item> {
        let agent = spec.name;
        let mut out = Vec::new();
        for dir in spec.skill_dirs {
            out.extend(self.skills(&self.rel(dir), agent));
        }
        for dir in spec.subagent_dirs {
            out.extend(self.markdown_items(&self.rel(dir), Kind::Subagent, agent, &["md"]));
        }
        for dir in spec.command_dirs {
            out.extend(self.markdown_items(&self.rel(dir), Kind::Command, agent, &["md", "toml"]));
        }
        for file in spec.memory_files {
            out.extend(self.memory_file(&self.rel(file), agent));
        }
        for dir in spec.memory_dirs {
            for file in children(&self.rel(dir)) {
                if extension(&file) == "md" {
                    out.extend(self.memory_file(&file, agent));
                }
            }
        }
        for file in spec.mcp_json {
            let file = self.rel(file);
            let servers = read_json(&file).and_then(|v| v.get("mcpServers").cloned());
            out.extend(self.mcp_items(servers.as_ref(), &file, agent, None, |name| Removal::JsonKey {
                file: file.clone(),
                key_path: vec!["mcpServers".into(), name.into()],
            }));
        }
        for file in spec.mcp_toml {
            out.extend(self.mcp_from_toml(&self.rel(file), agent, spec.cli));
        }
        for dir in spec.plugin_dirs {
            out.extend(self.plugin_folders(&self.rel(dir), agent));
        }
        for file in spec.hook_files {
            out.extend(self.hooks(&self.rel(file), agent));
        }
        out
    }

    // MARK: kinds

    pub fn skills(&self, dir: &Path, agent: &str) -> Vec<Item> {
        children(dir)
            .into_iter()
            .filter_map(|folder| {
                let file = folder.join("SKILL.md");
                if !file.exists() {
                    return None;
                }
                let meta = frontmatter::parse(&head(&file));
                let name = meta.get("name").cloned().unwrap_or_else(|| file_name(&folder));
                let description = meta.get("description").cloned().unwrap_or_default();
                Some(Item {
                    tokens: tokens(&name) + tokens(&description),
                    summary: description,
                    bytes: size(&folder),
                    notes: link_note(&folder).into_iter().collect(),
                    ..Item::new(agent, Kind::Skill, name, Some(folder.clone()), Removal::Trash { path: folder })
                })
            })
            .collect()
    }

    pub fn markdown_items(&self, dir: &Path, kind: Kind, agent: &str, extensions: &[&str]) -> Vec<Item> {
        // shortcut: top level only; namespaced sub-folders of commands are not listed yet.
        children(dir)
            .into_iter()
            .filter(|file| file.is_file() && extensions.contains(&extension(file).as_str()))
            .map(|file| {
                let meta = frontmatter::parse(&head(&file));
                let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let name = meta.get("name").cloned().unwrap_or(stem);
                let description = meta.get("description").cloned().unwrap_or_default();
                Item {
                    tokens: tokens(&name) + tokens(&description),
                    summary: description,
                    bytes: size(&file),
                    notes: link_note(&file).into_iter().collect(),
                    ..Item::new(agent, kind, name, Some(file.clone()), Removal::Trash { path: file })
                }
            })
            .collect()
    }

    pub fn memory_file(&self, file: &Path, agent: &str) -> Option<Item> {
        let text = fs::read_to_string(file).ok()?;
        let mut chars = text.chars().count();
        // `@path` lines import other files (CLAUDE.md / GEMINI.md syntax); they load with this one.
        for line in text.lines() {
            if let Some(reference) = line.strip_prefix('@') {
                let reference = reference.trim();
                let target = match reference.strip_prefix("~/") {
                    Some(rest) => self.rel(rest),
                    None => file.parent().map(|dir| join_rel(dir, reference)).unwrap_or_default(),
                };
                chars += fs::read_to_string(target).map(|t| t.chars().count()).unwrap_or(0);
            }
        }
        Some(Item {
            summary: first_line(&text),
            bytes: size(file),
            tokens: (chars / 4) as u64,
            ..Item::new(agent, Kind::Memory, file_name(file), Some(file.to_path_buf()), Removal::Trash { path: file.to_path_buf() })
        })
    }

    pub fn mcp_items(
        &self,
        servers: Option<&Value>,
        file: &Path,
        agent: &str,
        project: Option<&str>,
        removal: impl Fn(&str) -> Removal,
    ) -> Vec<Item> {
        let Some(Value::Object(map)) = servers else { return vec![] };
        let mut names: Vec<&String> = map.keys().collect();
        names.sort();
        names
            .into_iter()
            .map(|name| {
                let item = Item {
                    summary: mcp_summary(&map[name]),
                    per_session: project.is_none(),
                    notes: project.map(|p| Note::ProjectScope { project: p.to_string() }).into_iter().collect(),
                    ..Item::new(agent, Kind::Mcp, name.clone(), Some(file.to_path_buf()), removal(name))
                };
                match project {
                    Some(p) => item.tagged(p),
                    None => item,
                }
            })
            .collect()
    }

    fn mcp_from_toml(&self, file: &Path, agent: &str, cli: Option<&str>) -> Vec<Item> {
        let Ok(text) = fs::read_to_string(file) else { return vec![] };
        // shortcut: reads `[mcp_servers.<name>]` headers only, not a full TOML parser; quoted names with dots are skipped.
        let mut names: Vec<String> = Vec::new();
        for line in text.lines() {
            let header = line.trim();
            let Some(inner) = header.strip_prefix("[mcp_servers.").and_then(|h| h.strip_suffix(']')) else { continue };
            let name = inner.trim_matches(|c| c == '"' || c == '\'').to_string();
            if !name.contains('.') && !names.contains(&name) {
                names.push(name);
            }
        }
        names
            .into_iter()
            .map(|name| {
                let removal = match cli {
                    Some(tool) => Removal::Cli { argv: argv(&[tool, "mcp", "remove", name.as_str()]), cwd: None },
                    None => Removal::Manual { file: file.to_path_buf() },
                };
                Item { summary: file.display().to_string(), ..Item::new(agent, Kind::Mcp, name, Some(file.to_path_buf()), removal) }
            })
            .collect()
    }

    fn plugin_folders(&self, dir: &Path, agent: &str) -> Vec<Item> {
        children(dir)
            .into_iter()
            .filter(|p| p.is_dir())
            .map(|folder| {
                let inventory = self.plugin_inventory(&folder);
                let mut notes = inventory.notes();
                notes.push(Note::FolderRefs);
                Item {
                    summary: manifest_description(&folder).unwrap_or_default(),
                    bytes: size(&folder),
                    tokens: inventory.tokens,
                    notes,
                    ..Item::new(agent, Kind::Plugin, file_name(&folder), Some(folder.clone()), Removal::Trash { path: folder })
                }
            })
            .collect()
    }

    fn hooks(&self, file: &Path, agent: &str) -> Vec<Item> {
        let Some(json) = read_json(file) else { return vec![] };
        let mut by_event = BTreeMap::new();
        collect_hooks(&json, &[], None, &mut by_event);
        by_event
            .into_iter()
            .map(|(event_path, commands)| {
                let name = event_path.last().cloned().unwrap_or_else(|| "hook".into());
                let tag = event_path.join("/");
                Item {
                    summary: commands.join("\n"),
                    notes: vec![Note::Hooks { count: commands.len(), location: event_path.join(" › ") }],
                    ..Item::new(
                        agent,
                        Kind::Hook,
                        name,
                        Some(file.to_path_buf()),
                        Removal::JsonKey { file: file.to_path_buf(), key_path: event_path },
                    )
                }
                .tagged(&tag)
            })
            .collect()
    }

    // MARK: plugins

    /// Counts what a plugin folder adds to the context, using the default layout (skills/, agents/, commands/, hooks/).
    pub fn plugin_inventory(&self, root: &Path) -> PluginInventory {
        // shortcut: custom component paths declared in plugin.json are not followed.
        let skills = self.skills(&root.join("skills"), "");
        let agents = self.markdown_items(&root.join("agents"), Kind::Subagent, "", &["md"]);
        let commands = self.markdown_items(&root.join("commands"), Kind::Command, "", &["md"]);
        let mut events = BTreeMap::new();
        for file in [root.join("hooks").join("hooks.json"), root.join("hooks.json")] {
            if let Some(json) = read_json(&file) {
                collect_hooks(&json, &[], None, &mut events);
            }
        }
        PluginInventory {
            tokens: skills.iter().chain(&agents).chain(&commands).map(|i| i.tokens).sum(),
            skills: skills.len(),
            agents: agents.len(),
            commands: commands.len(),
            hooks: events.values().map(Vec::len).sum(),
        }
    }
}

/// Finds `{"command": "..."}` objects and groups them by the event key above them.
/// Works for Claude/Gemini (`hooks.Event[].hooks[]`), Cursor (`hooks.event[]`) and Antigravity (`plugin.Event[]`).
pub fn collect_hooks(node: &Value, path: &[String], event: Option<&[String]>, out: &mut BTreeMap<Vec<String>, Vec<String>>) {
    match node {
        Value::Object(map) => {
            if let (Some(Value::String(command)), Some(event)) = (map.get("command"), event) {
                out.entry(event.to_vec()).or_default().push(command.clone());
                return;
            }
            for (key, value) in map {
                let mut child = path.to_vec();
                child.push(key.clone());
                let is_event = value.is_array() && key != "hooks";
                collect_hooks(value, &child, if is_event { Some(child.as_slice()) } else { event }, out);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_hooks(value, path, event, out);
            }
        }
        _ => {}
    }
}

pub fn mcp_summary(config: &Value) -> String {
    for key in ["url", "serverUrl", "httpUrl"] {
        if let Some(url) = config.get(key).and_then(Value::as_str) {
            return url.to_string();
        }
    }
    let command = config.get("command").and_then(Value::as_str).unwrap_or_default();
    let args: Vec<&str> = config.get("args").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    [command.to_string(), args.join(" ")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}

pub fn manifest_description(root: &Path) -> Option<String> {
    let candidates = [".claude-plugin/plugin.json", "plugin.json", "gemini-extension.json", ".cursor-plugin/plugin.json", ".codex-plugin/plugin.json", "package.json"];
    candidates.iter().find_map(|name| {
        read_json(&join_rel(root, name))?.get("description")?.as_str().map(str::to_string)
    })
}

// MARK: file helpers

pub fn join_rel(base: &Path, relative: &str) -> PathBuf {
    relative.split('/').filter(|c| !c.is_empty()).fold(base.to_path_buf(), |path, part| path.join(part))
}

/// Visible children, sorted by name. Non-UTF-8 names are skipped because the UI can't show them.
pub fn children(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_str().is_some() && !file_name(p).starts_with('.'))
        .collect();
    paths.sort();
    paths
}

pub fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

fn link_note(path: &Path) -> Option<Note> {
    if !is_symlink(path) {
        return None;
    }
    fs::read_link(path).ok().map(|target| Note::LinkTo { target: target.display().to_string() })
}

/// Bytes on disk. Symlinks count as 0 so shared folders are not counted twice.
pub fn size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 0 };
    if meta.file_type().is_symlink() {
        return 0;
    }
    if meta.is_file() {
        return allocated(&meta);
    }
    let Ok(entries) = fs::read_dir(path) else { return 0 };
    entries.filter_map(Result::ok).map(|e| size(&e.path())).sum()
}

#[cfg(unix)]
fn allocated(meta: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

#[cfg(not(unix))]
fn allocated(meta: &fs::Metadata) -> u64 {
    meta.len()
}

/// First 8 KB of a file: enough for frontmatter without reading large files.
pub fn head(file: &Path) -> String {
    let mut buffer = Vec::new();
    if let Ok(f) = fs::File::open(file) {
        let _ = f.take(8192).read_to_end(&mut buffer);
    }
    String::from_utf8_lossy(&buffer).into_owned()
}

pub fn read_json(file: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(file).ok()?).ok()
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(|l| l.trim().trim_start_matches('#').trim())
        .find(|l| !l.is_empty() && *l != "---")
        .unwrap_or_default()
        .to_string()
}

pub fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

// shortcut: ~4 characters per token; swap in a real tokenizer if exact numbers matter.
pub fn tokens(text: &str) -> u64 {
    (text.chars().count() as u64 + 3) / 4
}
