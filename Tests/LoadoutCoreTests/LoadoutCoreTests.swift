import Foundation
import Testing
@testable import LoadoutCore

@Suite struct FrontmatterTests {
    @Test func inlineQuotedAndBlockValues() {
        let text = """
        ---
        name: demo
        description: |
          line one
          line two
        model: 'opus'
        ---
        body
        """
        let meta = Frontmatter.parse(text)
        #expect(meta["name"] == "demo")
        #expect(meta["description"] == "line one line two")
        #expect(meta["model"] == "opus")
    }

    @Test func noFrontmatter() {
        #expect(Frontmatter.parse("# Title\nname: not frontmatter").isEmpty)
    }
}

/// Builds a fake home folder with known agent layouts, so scanners run against fixtures, not the real Mac.
struct FakeHome {
    let root: URL

    init() throws {
        root = FileManager.default.temporaryDirectory.appendingPathComponent("loadout-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    func write(_ relative: String, _ text: String) throws {
        let file = root.appendingPathComponent(relative)
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: file, atomically: true, encoding: .utf8)
    }
}

@Suite struct ScannerTests {
    func fixture() throws -> FakeHome {
        let home = try FakeHome()
        let cache = home.root.appendingPathComponent(".claude/plugins/cache/mkt/tool").path
        try home.write(".claude/skills/demo/SKILL.md", "---\nname: demo\ndescription: Does a demo thing\n---\nbody")
        try home.write(".claude/agents/reviewer.md", "---\nname: reviewer\ndescription: Reviews code\n---\n")
        try home.write(".claude/CLAUDE.md", "# Rules\nBe brief.\n@EXTRA.md\n")
        try home.write(".claude/EXTRA.md", String(repeating: "x", count: 400))
        try home.write(".claude/settings.json", """
        {"enabledPlugins": {"tool@mkt": true},
         "statusLine": {"type": "command", "command": "not-a-hook"},
         "hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "echo hi"}]}]}}
        """)
        try home.write(".claude/plugins/installed_plugins.json", """
        {"version": 2, "plugins": {"tool@mkt": [{"installPath": "\(cache)/2.0.0", "version": "2.0.0"}]}}
        """)
        try home.write(".claude/plugins/cache/mkt/tool/2.0.0/skills/s1/SKILL.md", "---\nname: s1\ndescription: Plugin skill\n---\n")
        try home.write(".claude/plugins/cache/mkt/tool/2.0.0/hooks/hooks.json",
                       #"{"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "a"}]}]}}"#)
        try home.write(".claude/plugins/cache/mkt/tool/1.0.0/skills/s1/SKILL.md", "old")
        try home.write(".claude/plugins/known_marketplaces.json", """
        {"mkt": {"source": {"source": "github", "repo": "me/mkt"}},
         "stale": {"source": {"source": "github", "repo": "me/stale"}}}
        """)
        try home.write(".claude.json", """
        {"mcpServers": {"srv": {"command": "node", "args": ["x.js"]}},
         "projects": {"/tmp/proj": {"mcpServers": {"local1": {"url": "https://example.com/mcp"}}}}}
        """)
        try home.write(".claude/projects/-tmp-proj/memory/MEMORY.md", "- [A](a.md)\n")
        try home.write(".claude/projects/-tmp-proj/memory/a.md", "fact")
        try home.write(".cursor/mcp.json", #"{"mcpServers": {"cm": {"url": "https://x"}}}"#)
        try home.write(".cursor/hooks.json", #"{"version": 1, "hooks": {"preToolUse": [{"command": "rtk", "matcher": "Shell"}]}}"#)
        try home.write(".codex/config.toml", "[mcp_servers.docs]\ncommand = \"x\"\n\n[mcp_servers.docs.env]\nA = \"1\"\n")
        try home.write(".codex/AGENTS.md", "Codex rules")
        return home
    }

    @Test func scansClaudeLayout() throws {
        let home = try fixture()
        let items = InventoryScanner(home: home.root).scanAll()
        func find(_ kind: Kind, _ name: String) -> Item? { items.first { $0.kind == kind && $0.name == name } }

        let skill = try #require(find(.skill, "demo"))
        #expect(skill.summary == "Does a demo thing")
        #expect(skill.tokens > 0)
        #expect(find(.subagent, "reviewer") != nil)

        let memory = try #require(find(.memory, "CLAUDE.md"))
        #expect(memory.tokens >= 100, "@EXTRA.md import should be counted")

        let hooks = items.filter { $0.kind == .hook && $0.agent == "Claude Code" }
        #expect(hooks.count == 1, "statusLine is not a hook")
        #expect(hooks.first?.summary == "echo hi")
        #expect(hooks.first?.removal == .jsonKey(file: home.root.appendingPathComponent(".claude/settings.json"), path: ["hooks", "PreToolUse"]))

        let plugin = try #require(find(.plugin, "tool@mkt"))
        #expect(plugin.enabled == true)
        #expect(plugin.note == "v2.0.0 · 1 skill · 1 hook")
        #expect(plugin.removal == .cli(["claude", "plugin", "uninstall", "tool@mkt"]))

        let leftovers = items.filter { $0.kind == .leftover }
        #expect(leftovers.map(\.name) == ["tool 1.0.0"])

        #expect(find(.marketplace, "mkt")?.note == nil)
        #expect(find(.marketplace, "stale")?.note != nil)

        #expect(find(.mcp, "srv")?.removal == .cli(["claude", "mcp", "remove", "srv", "-s", "user"]))
        let local = try #require(find(.mcp, "local1"))
        #expect(local.perSession == false)
        #expect(local.removal == .cli(["claude", "mcp", "remove", "local1", "-s", "local"], cwd: URL(fileURLWithPath: "/tmp/proj")))

        let projectMemory = try #require(items.first { $0.kind == .memory && $0.perSession == false })
        #expect(projectMemory.summary == "1 memory for this project")
    }

    @Test func scansOtherAgents() throws {
        let home = try fixture()
        let items = InventoryScanner(home: home.root).scanAll()

        let cursorMCP = try #require(items.first { $0.agent == "Cursor" && $0.kind == .mcp })
        #expect(cursorMCP.removal == .jsonKey(file: home.root.appendingPathComponent(".cursor/mcp.json"), path: ["mcpServers", "cm"]))
        #expect(items.first { $0.agent == "Cursor" && $0.kind == .hook }?.name == "preToolUse")

        let codexMCP = items.filter { $0.agent == "Codex" && $0.kind == .mcp }
        #expect(codexMCP.map(\.name) == ["docs"], "[mcp_servers.docs.env] is not a separate server")
        #expect(items.contains { $0.agent == "Codex" && $0.kind == .memory && $0.name == "AGENTS.md" })
        #expect(!items.contains { $0.agent == "Gemini CLI" }, "missing agents are hidden")
    }
}

@Suite struct ActionTests {
    @Test func removeJSONKeyKeepsOtherKeysAndBacksUp() throws {
        let home = try FakeHome()
        try home.write(".cursor/mcp.json", #"{"mcpServers": {"a": {"url": "https://a"}, "b": {"url": "https://b"}}, "other": 1}"#)
        let file = home.root.appendingPathComponent(".cursor/mcp.json")
        let backups = home.root.appendingPathComponent("backups")
        let actions = Actions(home: home.root, backupRoot: backups)

        try actions.removeJSONKey(file: file, path: ["mcpServers", "a"])

        let json = try #require(try JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any])
        let servers = try #require(json["mcpServers"] as? [String: Any])
        #expect(servers.keys.sorted() == ["b"])
        #expect(json["other"] as? Int == 1)

        let backupFolders = try FileManager.default.contentsOfDirectory(atPath: backups.path)
        #expect(backupFolders.count == 1)
        let saved = try FileManager.default.contentsOfDirectory(atPath: backups.appendingPathComponent(backupFolders[0]).path)
        #expect(saved == [".cursor__mcp.json"])
    }

    @Test func missingToolGivesClearError() async {
        let actions = Actions(home: URL(fileURLWithPath: "/nonexistent"))
        await #expect(throws: ActionError.self) {
            try await actions.run(["definitely-not-a-real-tool-xyz"], cwd: nil)
        }
    }
}
