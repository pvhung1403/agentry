use crate::model::{Item, Kind, Note, Removal, Toggle};
use crate::scanner::{argv, children, file_name, manifest_description, read_json, size, Scanner};
use crate::specs::CLAUDE;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Claude Code keeps plugins, marketplaces, MCP servers and project memory in its own files.
impl Scanner {
    pub fn scan_claude_extras(&self) -> Vec<Item> {
        let agent = CLAUDE;
        let plugins_dir = self.rel(".claude/plugins");
        let mut out = Vec::new();

        // Installed plugins: plugins/installed_plugins.json, on/off state in settings.json.
        let installed = read_json(&plugins_dir.join("installed_plugins.json")).and_then(|v| v.get("plugins").cloned());
        let settings = read_json(&self.rel(".claude/settings.json"));
        let enabled_plugins = settings.as_ref().and_then(|s| s.get("enabledPlugins"));
        let mut live_paths = HashSet::new();
        let mut used_marketplaces = HashSet::new();

        if let Some(Value::Object(plugins)) = &installed {
            let mut ids: Vec<&String> = plugins.keys().collect();
            ids.sort();
            for id in ids {
                let entries = plugins[id].as_array().cloned().unwrap_or_default();
                for entry in &entries {
                    if let Some(path) = entry.get("installPath").and_then(Value::as_str) {
                        live_paths.insert(canonical(Path::new(path)));
                    }
                }
                let Some(entry) = entries.first() else { continue };
                let Some(path) = entry.get("installPath").and_then(Value::as_str) else { continue };
                if let Some(marketplace) = id.rsplit('@').next() {
                    used_marketplaces.insert(marketplace.to_string());
                }
                let folder = PathBuf::from(path);
                let inventory = self.plugin_inventory(&folder);
                let mut notes: Vec<Note> = entry
                    .get("version")
                    .and_then(Value::as_str)
                    .map(|v| Note::Version { value: v.to_string() })
                    .into_iter()
                    .collect();
                notes.extend(inventory.notes());
                let enabled = enabled_plugins.and_then(|m| m.get(id.as_str())).and_then(Value::as_bool).unwrap_or(false);
                out.push(Item {
                    summary: manifest_description(&folder).unwrap_or_default(),
                    bytes: size(&folder),
                    tokens: inventory.tokens,
                    enabled: Some(enabled),
                    toggle: Some(Toggle {
                        enable: argv(&["claude", "plugin", "enable", id.as_str()]),
                        disable: argv(&["claude", "plugin", "disable", id.as_str()]),
                    }),
                    notes,
                    ..Item::new(
                        agent,
                        Kind::Plugin,
                        id.clone(),
                        Some(folder),
                        Removal::Cli { argv: argv(&["claude", "plugin", "uninstall", id.as_str()]), cwd: None },
                    )
                });
            }
        }

        // Leftovers: cache/<marketplace>/<plugin>/<version> folders no installed plugin points to.
        for marketplace in children(&plugins_dir.join("cache")).into_iter().filter(|p| p.is_dir()) {
            for plugin in children(&marketplace).into_iter().filter(|p| p.is_dir()) {
                for version in children(&plugin).into_iter().filter(|p| p.is_dir()) {
                    if live_paths.contains(&canonical(&version)) {
                        continue;
                    }
                    out.push(Item {
                        bytes: size(&version),
                        notes: vec![Note::Leftover, Note::RestartAfterClean],
                        ..Item::new(
                            agent,
                            Kind::Leftover,
                            format!("{} {}", file_name(&plugin), file_name(&version)),
                            Some(version.clone()),
                            Removal::Trash { path: version },
                        )
                    });
                }
            }
        }

        // Marketplaces: the git repos Claude installs plugins from.
        if let Some(Value::Object(marketplaces)) = read_json(&plugins_dir.join("known_marketplaces.json")) {
            let mut names: Vec<&String> = marketplaces.keys().collect();
            names.sort();
            for name in names {
                let info = &marketplaces[name];
                let location = info
                    .get("installLocation")
                    .and_then(Value::as_str)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| plugins_dir.join("marketplaces").join(name));
                let source = info.get("source");
                let summary = ["repo", "url", "path"]
                    .iter()
                    .find_map(|k| source.and_then(|s| s.get(*k)).and_then(Value::as_str))
                    .unwrap_or_default()
                    .to_string();
                // The official marketplace is a default, so it is never flagged as unused.
                let unused = !used_marketplaces.contains(name.as_str()) && name != "claude-plugins-official";
                out.push(Item {
                    summary,
                    bytes: size(&location),
                    notes: if unused { vec![Note::UnusedMarketplace] } else { vec![] },
                    ..Item::new(
                        agent,
                        Kind::Marketplace,
                        name.clone(),
                        Some(location),
                        Removal::Cli { argv: argv(&["claude", "plugin", "marketplace", "remove", name.as_str()]), cwd: None },
                    )
                });
            }
        }

        // MCP servers: ~/.claude.json, user scope at the top, local scope per project.
        let config_file = self.rel(".claude.json");
        if let Some(config) = read_json(&config_file) {
            out.extend(self.mcp_items(config.get("mcpServers"), &config_file, agent, None, |name| Removal::Cli {
                argv: argv(&["claude", "mcp", "remove", name, "-s", "user"]),
                cwd: None,
            }));
            if let Some(Value::Object(projects)) = config.get("projects") {
                let mut paths: Vec<&String> = projects.keys().collect();
                paths.sort();
                for project in paths {
                    let servers = projects[project].get("mcpServers");
                    out.extend(self.mcp_items(servers, &config_file, agent, Some(project.as_str()), |name| Removal::Cli {
                        argv: argv(&["claude", "mcp", "remove", name, "-s", "local"]),
                        cwd: Some(PathBuf::from(project)),
                    }));
                }
            }
        }

        // Auto-memory: ~/.claude/projects/<encoded path>/memory/. MEMORY.md loads in that project only.
        let encoded_home = format!("{}-", encode_path(&self.home));
        for project in children(&self.rel(".claude/projects")) {
            let memory = project.join("memory");
            if !memory.is_dir() {
                continue;
            }
            let index = fs::read_to_string(memory.join("MEMORY.md")).unwrap_or_default();
            let count = children(&memory)
                .iter()
                .filter(|f| f.extension().is_some_and(|e| e == "md") && file_name(f) != "MEMORY.md")
                .count();
            let folder = file_name(&project);
            let name = folder.strip_prefix(&encoded_home).unwrap_or(&folder);
            out.push(Item {
                bytes: size(&memory),
                tokens: (index.chars().count() / 4) as u64,
                per_session: false,
                notes: vec![Note::ProjectMemory { count }],
                ..Item::new(
                    agent,
                    Kind::Memory,
                    if name.is_empty() { "~" } else { name },
                    Some(memory.clone()),
                    Removal::Trash { path: memory },
                )
            });
        }
        out
    }
}

/// Claude names project folders by replacing every non-alphanumeric character of the path with "-".
fn encode_path(path: &Path) -> String {
    path.to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect()
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
