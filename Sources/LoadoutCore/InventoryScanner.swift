import Foundation

/// Reads every agent's folders and builds the item list. Read-only: it never changes a file.
public struct InventoryScanner: Sendable {
    public let home: URL

    public init(home: URL = FileManager.default.homeDirectoryForCurrentUser) {
        self.home = home
    }

    public func scanAll() -> [Item] {
        var items: [Item] = []
        for spec in AgentSpec.all where exists(url(spec.root)) {
            items += scan(spec)
            if spec.name == AgentSpec.claudeName { items += scanClaudeExtras() }
        }
        return items
    }

    func scan(_ spec: AgentSpec) -> [Item] {
        let agent = spec.name
        var out: [Item] = []
        for dir in spec.skillDirs { out += skills(in: url(dir), agent: agent) }
        for dir in spec.subagentDirs { out += markdownItems(in: url(dir), kind: .subagent, agent: agent) }
        for dir in spec.commandDirs { out += markdownItems(in: url(dir), kind: .command, agent: agent, extensions: ["md", "toml"]) }
        for file in spec.memoryFiles { if let item = memoryFile(url(file), agent: agent) { out.append(item) } }
        for dir in spec.memoryDirs {
            for file in children(url(dir)) where file.pathExtension == "md" {
                if let item = memoryFile(file, agent: agent) { out.append(item) }
            }
        }
        for file in spec.mcpJSON {
            let servers = (readJSON(url(file)) as? [String: Any])?["mcpServers"] as? [String: Any] ?? [:]
            out += mcpItems(servers, file: url(file), agent: agent) { .jsonKey(file: url(file), path: ["mcpServers", $0]) }
        }
        for file in spec.mcpTOML { out += mcpFromTOML(url(file), agent: agent, cli: spec.cli) }
        for dir in spec.pluginDirs { out += pluginFolders(in: url(dir), agent: agent) }
        for file in spec.hookFiles { out += hooks(in: url(file), agent: agent) }
        return out
    }

    // MARK: - Kinds

    func skills(in dir: URL, agent: String) -> [Item] {
        children(dir).compactMap { folder in
            let file = folder.appendingPathComponent("SKILL.md")
            guard exists(file) else { return nil }
            let meta = Frontmatter.parse(head(file))
            let name = meta["name"] ?? folder.lastPathComponent
            let description = meta["description"] ?? ""
            return Item(agent: agent, kind: .skill, name: name, summary: description, path: folder,
                        bytes: size(folder), tokens: Self.tokens(name + description),
                        removal: .trash(folder), note: linkNote(folder))
        }
    }

    func markdownItems(in dir: URL, kind: Kind, agent: String, extensions: Set<String> = ["md"]) -> [Item] {
        // shortcut: top level only; namespaced sub-folders of commands are not listed yet.
        children(dir).filter { extensions.contains($0.pathExtension) }.map { file in
            let meta = Frontmatter.parse(head(file))
            let name = meta["name"] ?? file.deletingPathExtension().lastPathComponent
            let description = meta["description"] ?? ""
            return Item(agent: agent, kind: kind, name: name, summary: description, path: file,
                        bytes: size(file), tokens: Self.tokens(name + description),
                        removal: .trash(file), note: linkNote(file))
        }
    }

    func memoryFile(_ file: URL, agent: String) -> Item? {
        guard let text = try? String(contentsOf: file, encoding: .utf8) else { return nil }
        var chars = text.count
        // `@path` lines import other files (CLAUDE.md / GEMINI.md syntax); they load with this one.
        for line in text.split(separator: "\n") where line.hasPrefix("@") {
            let ref = line.dropFirst().trimmingCharacters(in: .whitespaces)
            let target = ref.hasPrefix("~/")
                ? home.appendingPathComponent(String(ref.dropFirst(2)))
                : file.deletingLastPathComponent().appendingPathComponent(ref)
            chars += (try? String(contentsOf: target, encoding: .utf8))?.count ?? 0
        }
        return Item(agent: agent, kind: .memory, name: file.lastPathComponent, summary: firstLine(text),
                    path: file, bytes: size(file), tokens: chars / 4, removal: .trash(file))
    }

    func mcpItems(_ servers: [String: Any], file: URL, agent: String, project: String? = nil,
                  removal: (String) -> Removal) -> [Item] {
        servers.keys.sorted().map { name in
            let config = servers[name] as? [String: Any] ?? [:]
            var summary = Self.mcpSummary(config)
            if let project { summary = "Project \(project) · " + summary }
            return Item(agent: agent, kind: .mcp, name: name, summary: summary, path: file,
                        perSession: project == nil, removal: removal(name))
        }
    }

    static func mcpSummary(_ config: [String: Any]) -> String {
        for key in ["url", "serverUrl", "httpUrl"] {
            if let url = config[key] as? String { return url }
        }
        let command = config["command"] as? String ?? ""
        let args = (config["args"] as? [String] ?? []).joined(separator: " ")
        return [command, args].filter { !$0.isEmpty }.joined(separator: " ")
    }

    func mcpFromTOML(_ file: URL, agent: String, cli: String?) -> [Item] {
        guard let text = try? String(contentsOf: file, encoding: .utf8) else { return [] }
        // shortcut: reads `[mcp_servers.<name>]` headers only, not a full TOML parser; quoted names with dots are skipped.
        var names: [String] = []
        for line in text.split(separator: "\n") {
            let header = line.trimmingCharacters(in: .whitespaces)
            guard header.hasPrefix("[mcp_servers."), header.hasSuffix("]") else { continue }
            let name = String(header.dropFirst("[mcp_servers.".count).dropLast())
                .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
            if !name.contains("."), !names.contains(name) { names.append(name) }
        }
        return names.map { name in
            let removal: Removal = cli.map { .cli([$0, "mcp", "remove", name]) }
                ?? .unsupported("Edit \(file.path) to remove this server.")
            return Item(agent: agent, kind: .mcp, name: name, summary: file.path, path: file, removal: removal)
        }
    }

    func pluginFolders(in dir: URL, agent: String) -> [Item] {
        children(dir).filter(isDirectory).map { folder in
            let inventory = pluginInventory(folder)
            return Item(agent: agent, kind: .plugin, name: folder.lastPathComponent,
                        summary: manifestDescription(folder) ?? inventory.summary, path: folder,
                        bytes: size(folder), tokens: inventory.tokens, removal: .trash(folder),
                        note: [inventory.summary, "Removing the folder can leave references in the agent's config."]
                            .filter { !$0.isEmpty }.joined(separator: " · "))
        }
    }

    func hooks(in file: URL, agent: String) -> [Item] {
        guard let json = readJSON(file) else { return [] }
        var byEvent: [[String]: [String]] = [:]
        Self.collectHooks(json, path: [], event: nil, into: &byEvent)
        return byEvent.keys.sorted { $0.joined() < $1.joined() }.map { eventPath in
            let commands = byEvent[eventPath] ?? []
            return Item(agent: agent, kind: .hook, name: eventPath.last ?? "hook",
                        summary: commands.joined(separator: "\n"), path: file,
                        removal: .jsonKey(file: file, path: eventPath),
                        note: "\(commands.count) command\(commands.count == 1 ? "" : "s") · " + eventPath.joined(separator: " › "))
        }
    }

    /// Finds `{"command": "..."}` objects and groups them by the event key above them.
    /// Works for Claude/Gemini (`hooks.Event[].hooks[]`), Cursor (`hooks.event[]`) and Antigravity (`plugin.Event[]`).
    static func collectHooks(_ node: Any, path: [String], event: [String]?, into out: inout [[String]: [String]]) {
        if let dict = node as? [String: Any] {
            if let command = dict["command"] as? String, let event {
                out[event, default: []].append(command)
                return
            }
            for (key, value) in dict {
                let isEvent = value is [Any] && key != "hooks"
                collectHooks(value, path: path + [key], event: isEvent ? path + [key] : event, into: &out)
            }
        } else if let array = node as? [Any] {
            for value in array { collectHooks(value, path: path, event: event, into: &out) }
        }
    }

    // MARK: - Plugins

    struct PluginInventory {
        var tokens = 0
        var summary = ""
    }

    /// Counts what a plugin folder adds to the context. Uses the default layout (skills/, agents/, commands/, hooks/).
    func pluginInventory(_ root: URL) -> PluginInventory {
        // shortcut: custom component paths declared in plugin.json are not followed.
        let skillItems = skills(in: root.appendingPathComponent("skills"), agent: "")
        let agentItems = markdownItems(in: root.appendingPathComponent("agents"), kind: .subagent, agent: "")
        let commandItems = markdownItems(in: root.appendingPathComponent("commands"), kind: .command, agent: "")
        var hookEvents: [[String]: [String]] = [:]
        for name in ["hooks/hooks.json", "hooks.json"] {
            if let json = readJSON(root.appendingPathComponent(name)) {
                Self.collectHooks(json, path: [], event: nil, into: &hookEvents)
            }
        }
        let hookCount = hookEvents.values.reduce(0) { $0 + $1.count }
        let parts = [(skillItems.count, "skill"), (agentItems.count, "agent"), (commandItems.count, "command"), (hookCount, "hook")]
            .filter { $0.0 > 0 }
            .map { "\($0.0) \($0.1)\($0.0 == 1 ? "" : "s")" }
        let tokens = (skillItems + agentItems + commandItems).reduce(0) { $0 + $1.tokens }
        return PluginInventory(tokens: tokens, summary: parts.joined(separator: " · "))
    }

    func manifestDescription(_ root: URL) -> String? {
        let candidates = [".claude-plugin/plugin.json", "plugin.json", "gemini-extension.json",
                          ".cursor-plugin/plugin.json", ".codex-plugin/plugin.json", "package.json"]
        for name in candidates {
            if let description = (readJSON(root.appendingPathComponent(name)) as? [String: Any])?["description"] as? String {
                return description
            }
        }
        return nil
    }

    // MARK: - File helpers

    func url(_ relative: String) -> URL { home.appendingPathComponent(relative) }

    func exists(_ url: URL) -> Bool { FileManager.default.fileExists(atPath: url.path) }

    func children(_ dir: URL) -> [URL] {
        let urls = (try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil,
                                                                 options: [.skipsHiddenFiles])) ?? []
        return urls.sorted { $0.lastPathComponent < $1.lastPathComponent }
    }

    func isDirectory(_ url: URL) -> Bool {
        var isDir: ObjCBool = false
        return FileManager.default.fileExists(atPath: url.path, isDirectory: &isDir) && isDir.boolValue
    }

    func isSymlink(_ url: URL) -> Bool {
        (try? url.resourceValues(forKeys: [.isSymbolicLinkKey]))?.isSymbolicLink == true
    }

    func linkNote(_ url: URL) -> String? {
        guard isSymlink(url), let target = try? FileManager.default.destinationOfSymbolicLink(atPath: url.path) else { return nil }
        return "Link to \(target)"
    }

    /// Bytes on disk. Symlinks count as 0 so shared folders are not counted twice.
    func size(_ url: URL) -> Int64 {
        if isSymlink(url) { return 0 }
        let keys: Set<URLResourceKey> = [.totalFileAllocatedSizeKey, .isRegularFileKey]
        guard isDirectory(url) else {
            return Int64((try? url.resourceValues(forKeys: keys))?.totalFileAllocatedSize ?? 0)
        }
        var total: Int64 = 0
        let walker = FileManager.default.enumerator(at: url, includingPropertiesForKeys: Array(keys))
        while let file = walker?.nextObject() as? URL {
            if let values = try? file.resourceValues(forKeys: keys), values.isRegularFile == true {
                total += Int64(values.totalFileAllocatedSize ?? 0)
            }
        }
        return total
    }

    /// First 8 KB of a file: enough for frontmatter without reading large files.
    func head(_ file: URL) -> String {
        guard let handle = try? FileHandle(forReadingFrom: file) else { return "" }
        defer { try? handle.close() }
        let data = (try? handle.read(upToCount: 8192)) ?? Data()
        return String(decoding: data, as: UTF8.self)
    }

    func readJSON(_ file: URL) -> Any? {
        guard let data = try? Data(contentsOf: file) else { return nil }
        return try? JSONSerialization.jsonObject(with: data)
    }

    func firstLine(_ text: String) -> String {
        text.split(separator: "\n")
            .map { $0.trimmingCharacters(in: CharacterSet(charactersIn: "# ").union(.whitespaces)) }
            .first { !$0.isEmpty && $0 != "---" } ?? ""
    }

    // shortcut: ~4 characters per token; swap in a real tokenizer if exact numbers matter.
    static func tokens(_ text: String) -> Int { (text.count + 3) / 4 }
}
