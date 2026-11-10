import Foundation

public enum Kind: String, CaseIterable, Sendable {
    case skill, plugin, memory, subagent, command, mcp, hook, marketplace, leftover

    public var title: String {
        switch self {
        case .skill: "Skills"
        case .plugin: "Plugins"
        case .memory: "Memory"
        case .subagent: "Subagents"
        case .command: "Commands"
        case .mcp: "MCP Servers"
        case .hook: "Hooks"
        case .marketplace: "Marketplaces"
        case .leftover: "Leftovers"
        }
    }

    public var symbol: String {
        switch self {
        case .skill: "sparkles"
        case .plugin: "puzzlepiece.extension"
        case .memory: "brain"
        case .subagent: "person.2"
        case .command: "terminal"
        case .mcp: "server.rack"
        case .hook: "link"
        case .marketplace: "storefront"
        case .leftover: "trash"
        }
    }
}

/// How an item is removed. The agent's own CLI is preferred so its state files stay consistent.
public enum Removal: Hashable, Sendable {
    case trash(URL)
    /// argv[0] is a tool name (e.g. "claude"), resolved to a path when run.
    case cli([String], cwd: URL? = nil)
    /// Deletes one key from a JSON config file, after saving a backup.
    case jsonKey(file: URL, path: [String])
    case unsupported(String)

    public var isSupported: Bool {
        if case .unsupported = self { return false }
        return true
    }

    public var explanation: String {
        switch self {
        case .trash(let url): "Moves \(url.path) to the Trash."
        case .cli(let argv, let cwd): "Runs `\(argv.joined(separator: " "))`" + (cwd.map { " in \($0.path)" } ?? "") + "."
        case .jsonKey(let file, let path): "Removes \(path.joined(separator: " › ")) from \(file.path). A backup is saved first."
        case .unsupported(let reason): reason
        }
    }
}

public struct ToggleCommands: Hashable, Sendable {
    public let enable: [String]
    public let disable: [String]
}

public struct Item: Identifiable, Hashable, Sendable {
    public let id: String
    public let agent: String
    public let kind: Kind
    public let name: String
    public let summary: String
    public let path: URL?
    public let bytes: Int64
    /// Estimated tokens this item adds to the context (skill descriptions, always-loaded memory).
    public let tokens: Int
    /// False when the tokens only load in one project, so they are left out of the per-session total.
    public let perSession: Bool
    /// nil when the item has no on/off switch.
    public let enabled: Bool?
    public let toggle: ToggleCommands?
    public let removal: Removal
    public let note: String?

    init(agent: String, kind: Kind, name: String, summary: String = "", path: URL? = nil,
         bytes: Int64 = 0, tokens: Int = 0, perSession: Bool = true, enabled: Bool? = nil,
         toggle: ToggleCommands? = nil, removal: Removal, note: String? = nil) {
        self.id = [agent, kind.rawValue, path?.path ?? "", name].joined(separator: "|")
        self.agent = agent
        self.kind = kind
        self.name = name
        self.summary = summary
        self.path = path
        self.bytes = bytes
        self.tokens = tokens
        self.perSession = perSession
        self.enabled = enabled
        self.toggle = toggle
        self.removal = removal
        self.note = note
    }

    /// Tokens counted toward "loaded every session": disabled items and project-only items don't count.
    public var sessionTokens: Int { perSession && enabled != false ? tokens : 0 }
}
