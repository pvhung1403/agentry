import Foundation

/// Claude Code keeps plugins, marketplaces, MCP servers and project memory in its own files.
extension InventoryScanner {
    func scanClaudeExtras() -> [Item] {
        let agent = AgentSpec.claudeName
        let pluginsDir = url(".claude/plugins")
        var out: [Item] = []

        // Installed plugins: ~/.claude/plugins/installed_plugins.json, on/off state in settings.json.
        let installed = (readJSON(pluginsDir.appendingPathComponent("installed_plugins.json")) as? [String: Any])?["plugins"]
            as? [String: Any] ?? [:]
        let enabledPlugins = (readJSON(url(".claude/settings.json")) as? [String: Any])?["enabledPlugins"]
            as? [String: Bool] ?? [:]
        var livePaths = Set<String>()
        var usedMarketplaces = Set<String>()

        for id in installed.keys.sorted() {
            let entries = installed[id] as? [[String: Any]] ?? []
            for entry in entries {
                if let path = entry["installPath"] as? String { livePaths.insert(canonical(URL(fileURLWithPath: path))) }
            }
            guard let entry = entries.first, let path = entry["installPath"] as? String else { continue }
            if let marketplace = id.split(separator: "@").last { usedMarketplaces.insert(String(marketplace)) }
            let folder = URL(fileURLWithPath: path)
            let inventory = pluginInventory(folder)
            let version = (entry["version"] as? String).map { "v\($0)" }
            out.append(Item(agent: agent, kind: .plugin, name: id, summary: manifestDescription(folder) ?? "",
                            path: folder, bytes: size(folder), tokens: inventory.tokens,
                            enabled: enabledPlugins[id] ?? false,
                            toggle: ToggleCommands(enable: ["claude", "plugin", "enable", id],
                                                   disable: ["claude", "plugin", "disable", id]),
                            removal: .cli(["claude", "plugin", "uninstall", id]),
                            note: [version, inventory.summary].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " · ")))
        }

        // Leftovers: cache/<marketplace>/<plugin>/<version> folders that no installed plugin points to.
        for marketplace in children(pluginsDir.appendingPathComponent("cache")) where isDirectory(marketplace) {
            for plugin in children(marketplace) where isDirectory(plugin) {
                for version in children(plugin) where isDirectory(version) && !livePaths.contains(canonical(version)) {
                    out.append(Item(agent: agent, kind: .leftover,
                                    name: "\(plugin.lastPathComponent) \(version.lastPathComponent)",
                                    summary: "Old plugin version left in the cache. Claude Code no longer uses it.",
                                    path: version, bytes: size(version), removal: .trash(version),
                                    note: "Restart running Claude Code sessions after cleaning."))
                }
            }
        }

        // Marketplaces (git repos Claude installs plugins from).
        let marketplaces = readJSON(pluginsDir.appendingPathComponent("known_marketplaces.json")) as? [String: Any] ?? [:]
        for name in marketplaces.keys.sorted() {
            let info = marketplaces[name] as? [String: Any] ?? [:]
            let location = (info["installLocation"] as? String).map { URL(fileURLWithPath: $0) }
                ?? pluginsDir.appendingPathComponent("marketplaces/\(name)")
            let source = info["source"] as? [String: Any] ?? [:]
            out.append(Item(agent: agent, kind: .marketplace, name: name,
                            summary: (source["repo"] ?? source["url"] ?? source["path"]) as? String ?? "",
                            path: location, bytes: size(location),
                            removal: .cli(["claude", "plugin", "marketplace", "remove", name]),
                            // The official marketplace is a default, so it is never flagged as unused.
                            note: usedMarketplaces.contains(name) || name == "claude-plugins-official"
                                ? nil : "No installed plugin comes from here."))
        }

        // MCP servers: ~/.claude.json, user scope at the top, local scope per project.
        let configFile = url(".claude.json")
        if let config = readJSON(configFile) as? [String: Any] {
            out += mcpItems(config["mcpServers"] as? [String: Any] ?? [:], file: configFile, agent: agent) {
                .cli(["claude", "mcp", "remove", $0, "-s", "user"])
            }
            let projects = config["projects"] as? [String: Any] ?? [:]
            for project in projects.keys.sorted() {
                let servers = (projects[project] as? [String: Any])?["mcpServers"] as? [String: Any] ?? [:]
                out += mcpItems(servers, file: configFile, agent: agent, project: project) {
                    .cli(["claude", "mcp", "remove", $0, "-s", "local"], cwd: URL(fileURLWithPath: project))
                }
            }
        }

        // Auto-memory: ~/.claude/projects/<encoded path>/memory/, MEMORY.md loads in that project.
        let encodedHome = home.path.replacingOccurrences(of: "/", with: "-") + "-"
        for project in children(url(".claude/projects")) {
            let memory = project.appendingPathComponent("memory")
            guard isDirectory(memory) else { continue }
            let index = (try? String(contentsOf: memory.appendingPathComponent("MEMORY.md"), encoding: .utf8)) ?? ""
            let count = children(memory).filter { $0.pathExtension == "md" && $0.lastPathComponent != "MEMORY.md" }.count
            let folderName = project.lastPathComponent
            let name = folderName.hasPrefix(encodedHome) ? String(folderName.dropFirst(encodedHome.count)) : folderName
            out.append(Item(agent: agent, kind: .memory, name: name.isEmpty ? "~" : name,
                            summary: "\(count) memor\(count == 1 ? "y" : "ies") for this project",
                            path: memory, bytes: size(memory), tokens: index.count / 4, perSession: false,
                            removal: .trash(memory), note: "Project auto-memory · MEMORY.md loads only in this project."))
        }
        return out
    }

    func canonical(_ url: URL) -> String { url.resolvingSymlinksInPath().standardizedFileURL.path }
}
