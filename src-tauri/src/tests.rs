use crate::actions::Actions;
use crate::frontmatter;
use crate::model::{Item, Kind, Note, Removal};
use crate::scanner::{join_rel, Scanner};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A fake home folder with known agent layouts, so tests never touch real config.
struct FakeHome {
    root: PathBuf,
}

impl FakeHome {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("agentry-test-{}-{n}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        FakeHome { root: fs::canonicalize(&root).unwrap() }
    }

    fn path(&self, relative: &str) -> PathBuf {
        join_rel(&self.root, relative)
    }

    fn write(&self, relative: &str, text: &str) {
        let file = self.path(relative);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    fn scan(&self) -> Vec<Item> {
        Scanner::new(self.root.clone()).scan_all()
    }

    fn actions(&self) -> Actions {
        Actions::new(self.root.clone(), self.path("backups"))
    }
}

impl Drop for FakeHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn find<'a>(items: &'a [Item], kind: Kind, name: &str) -> Option<&'a Item> {
    items.iter().find(|i| i.kind == kind && i.name == name)
}

fn fixture() -> FakeHome {
    let home = FakeHome::new();
    let install = serde_json::to_string(&home.path(".claude/plugins/cache/mkt/tool/2.0.0")).unwrap();
    home.write(".claude/skills/demo/SKILL.md", "---\nname: demo\ndescription: Does a demo thing\n---\nbody");
    home.write(".claude/agents/reviewer.md", "---\nname: reviewer\ndescription: Reviews code\n---\n");
    home.write(".claude/CLAUDE.md", "# Rules\nBe brief.\n@EXTRA.md\n");
    home.write(".claude/EXTRA.md", &"x".repeat(400));
    home.write(
        ".claude/settings.json",
        r#"{"enabledPlugins": {"tool@mkt": true},
            "statusLine": {"type": "command", "command": "not-a-hook"},
            "hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "echo hi"}]}]}}"#,
    );
    home.write(
        ".claude/plugins/installed_plugins.json",
        &format!(r#"{{"version": 2, "plugins": {{"tool@mkt": [{{"installPath": {install}, "version": "2.0.0"}}]}}}}"#),
    );
    home.write(".claude/plugins/cache/mkt/tool/2.0.0/skills/s1/SKILL.md", "---\nname: s1\ndescription: Plugin skill\n---\n");
    home.write(
        ".claude/plugins/cache/mkt/tool/2.0.0/hooks/hooks.json",
        r#"{"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "a"}]}]}}"#,
    );
    home.write(".claude/plugins/cache/mkt/tool/1.0.0/skills/s1/SKILL.md", "old");
    home.write(
        ".claude/plugins/known_marketplaces.json",
        r#"{"mkt": {"source": {"source": "github", "repo": "me/mkt"}},
            "stale": {"source": {"source": "github", "repo": "me/stale"}}}"#,
    );
    home.write(
        ".claude.json",
        r#"{"mcpServers": {"srv": {"command": "node", "args": ["x.js"]}},
            "projects": {"/tmp/proj": {"mcpServers": {"local1": {"url": "https://example.com/mcp"}}}}}"#,
    );
    home.write(".claude/projects/-tmp-proj/memory/MEMORY.md", "- [A](a.md)\n");
    home.write(".claude/projects/-tmp-proj/memory/a.md", "fact");
    home.write(".cursor/mcp.json", r#"{"mcpServers": {"cm": {"url": "https://x"}}}"#);
    home.write(".cursor/hooks.json", r#"{"version": 1, "hooks": {"preToolUse": [{"command": "rtk", "matcher": "Shell"}]}}"#);
    home.write(".codex/config.toml", "[mcp_servers.docs]\ncommand = \"x\"\n\n[mcp_servers.docs.env]\nA = \"1\"\n");
    home.write(".codex/AGENTS.md", "Codex rules");
    home
}

#[test]
fn frontmatter_inline_quoted_and_block_values() {
    let meta = frontmatter::parse("---\nname: demo\ndescription: |\n  line one\n  line two\nmodel: 'opus'\n---\nbody");
    assert_eq!(meta["name"], "demo");
    assert_eq!(meta["description"], "line one line two");
    assert_eq!(meta["model"], "opus");
    assert!(frontmatter::parse("# Title\nname: not frontmatter").is_empty());
}

#[test]
fn scans_claude_layout() {
    let home = fixture();
    let items = home.scan();

    let skill = find(&items, Kind::Skill, "demo").expect("skill");
    assert_eq!(skill.summary, "Does a demo thing");
    assert!(skill.tokens > 0);
    assert!(find(&items, Kind::Subagent, "reviewer").is_some());

    let memory = find(&items, Kind::Memory, "CLAUDE.md").expect("CLAUDE.md");
    assert!(memory.tokens >= 100, "@EXTRA.md import should be counted");

    let hooks: Vec<&Item> = items.iter().filter(|i| i.kind == Kind::Hook && i.agent == "Claude Code").collect();
    assert_eq!(hooks.len(), 1, "statusLine is not a hook");
    assert_eq!(hooks[0].summary, "echo hi");
    assert_eq!(
        hooks[0].removal,
        Removal::JsonKey { file: home.path(".claude/settings.json"), key_path: vec!["hooks".into(), "PreToolUse".into()] }
    );

    let plugin = find(&items, Kind::Plugin, "tool@mkt").expect("plugin");
    assert_eq!(plugin.enabled, Some(true));
    assert_eq!(
        plugin.notes,
        vec![Note::Version { value: "2.0.0".into() }, Note::Contents { skills: 1, agents: 0, commands: 0, hooks: 1 }]
    );
    assert_eq!(plugin.removal, Removal::Cli { argv: vec!["claude".into(), "plugin".into(), "uninstall".into(), "tool@mkt".into()], cwd: None });

    let leftovers: Vec<&str> = items.iter().filter(|i| i.kind == Kind::Leftover).map(|i| i.name.as_str()).collect();
    assert_eq!(leftovers, vec!["tool 1.0.0"]);

    assert!(find(&items, Kind::Marketplace, "mkt").unwrap().notes.is_empty());
    assert_eq!(find(&items, Kind::Marketplace, "stale").unwrap().notes, vec![Note::UnusedMarketplace]);

    let user_mcp = find(&items, Kind::Mcp, "srv").expect("user MCP");
    assert_eq!(
        user_mcp.removal,
        Removal::Cli { argv: ["claude", "mcp", "remove", "srv", "-s", "user"].map(String::from).to_vec(), cwd: None }
    );
    let local_mcp = find(&items, Kind::Mcp, "local1").expect("local MCP");
    assert!(!local_mcp.per_session);
    assert_eq!(
        local_mcp.removal,
        Removal::Cli {
            argv: ["claude", "mcp", "remove", "local1", "-s", "local"].map(String::from).to_vec(),
            cwd: Some(PathBuf::from("/tmp/proj"))
        }
    );

    let project_memory = items.iter().find(|i| i.kind == Kind::Memory && !i.per_session).expect("project memory");
    assert_eq!(project_memory.notes, vec![Note::ProjectMemory { count: 1 }]);
}

#[test]
fn scans_other_agents() {
    let home = fixture();
    let items = home.scan();

    let cursor_mcp = items.iter().find(|i| i.agent == "Cursor" && i.kind == Kind::Mcp).expect("Cursor MCP");
    assert_eq!(
        cursor_mcp.removal,
        Removal::JsonKey { file: home.path(".cursor/mcp.json"), key_path: vec!["mcpServers".into(), "cm".into()] }
    );
    assert_eq!(items.iter().find(|i| i.agent == "Cursor" && i.kind == Kind::Hook).unwrap().name, "preToolUse");

    let codex_mcp: Vec<&str> = items.iter().filter(|i| i.agent == "Codex" && i.kind == Kind::Mcp).map(|i| i.name.as_str()).collect();
    assert_eq!(codex_mcp, vec!["docs"], "[mcp_servers.docs.env] is not a separate server");
    assert!(items.iter().any(|i| i.agent == "Codex" && i.kind == Kind::Memory && i.name == "AGENTS.md"));
    assert!(!items.iter().any(|i| i.agent == "Gemini CLI"), "missing agents are hidden");
}

#[test]
fn ids_are_unique_when_events_repeat() {
    let home = FakeHome::new();
    home.write(".gemini/antigravity/skills/s/SKILL.md", "---\nname: s\n---\n");
    home.write(
        ".gemini/config/hooks.json",
        r#"{"a": {"SessionStart": [{"command": "x"}]}, "b": {"SessionStart": [{"command": "y"}]}}"#,
    );
    let items = home.scan();
    let hooks: Vec<&Item> = items.iter().filter(|i| i.kind == Kind::Hook).collect();
    assert_eq!(hooks.len(), 2);
    assert_ne!(hooks[0].id, hooks[1].id);
}

#[test]
fn remove_json_key_keeps_order_and_backs_up() {
    let home = FakeHome::new();
    home.write(
        ".cursor/mcp.json",
        r#"{"mcpServers": {"a": {"url": "https://a"}, "b": {"url": "https://b"}, "c": {"url": "https://c"}}, "other": 1}"#,
    );
    let file = home.path(".cursor/mcp.json");
    home.actions().remove_json_key(&file, &["mcpServers".into(), "b".into()]).unwrap();

    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let top: Vec<&String> = json.as_object().unwrap().keys().collect();
    assert_eq!(top, ["mcpServers", "other"]);
    let servers: Vec<&String> = json["mcpServers"].as_object().unwrap().keys().collect();
    assert_eq!(servers, ["a", "c"]);

    let backups: Vec<PathBuf> = fs::read_dir(home.path("backups")).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(backups.len(), 1);
    let saved: Vec<String> =
        fs::read_dir(&backups[0]).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(saved, [".cursor__mcp.json"]);
}

#[test]
fn missing_tool_gives_error_code() {
    let home = FakeHome::new();
    let error = home.actions().run(&["definitely-not-a-real-tool-xyz".to_string()], None).unwrap_err();
    assert_eq!(error, "tool-not-found:definitely-not-a-real-tool-xyz");
}

#[cfg(unix)]
#[test]
fn removing_a_linked_skill_keeps_the_shared_original() {
    let home = FakeHome::new();
    home.write(".agents/skills/shared/SKILL.md", "---\nname: shared\ndescription: Shared skill\n---\n");
    fs::create_dir_all(home.path(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(home.path(".agents/skills/shared"), home.path(".claude/skills/shared")).unwrap();

    let items = home.scan();
    let linked = items.iter().find(|i| i.agent == "Claude Code" && i.kind == Kind::Skill).expect("linked skill");
    assert_eq!(linked.bytes, 0, "links are not counted twice");
    assert!(matches!(linked.notes.first(), Some(Note::LinkTo { .. })));

    home.actions().remove(linked).unwrap();
    assert!(fs::symlink_metadata(home.path(".claude/skills/shared")).is_err(), "link removed");
    assert!(home.path(".agents/skills/shared/SKILL.md").exists(), "original kept");
}

#[test]
fn last_used_comes_from_claude_history() {
    let home = fixture();
    home.write(
        ".claude/projects/-tmp-proj/session.jsonl",
        concat!(
            r#"{"type":"user","timestamp":"2026-10-30T09:00:00.000Z","message":{"content":"<command-name>/demo</command-name>"}}"#, "\n",
            r#"{"type":"assistant","timestamp":"2026-11-01T10:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"demo"}}]}}"#, "\n",
            r#"{"type":"assistant","timestamp":"2026-11-02T10:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"tool:s1"}}]}}"#, "\n",
            r#"{"type":"assistant","timestamp":"2026-11-03T10:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Agent","input":{"subagent_type":"reviewer"}}]}}"#, "\n",
            r#"{"type":"assistant","timestamp":"2026-11-04T10:00:00.000Z","message":{"content":[{"type":"tool_use","name":"mcp__srv__do","input":{}}]}}"#, "\n",
        ),
    );
    // Subagent transcripts live in nested folders.
    home.write(
        ".claude/projects/-tmp-proj/abc/subagents/agent-1.jsonl",
        r#"{"type":"assistant","timestamp":"2026-11-05T10:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"tool:s1"}}]}}"#,
    );

    let mut items = home.scan();
    let usage = crate::usage::Usage::read(&home.path(".claude/projects"));
    usage.apply(&mut items);
    let last = |kind, name| find(&items, kind, name).and_then(|i| i.last_used.clone());

    assert_eq!(usage.since.as_deref(), Some("2026-10-30T09:00:00.000Z"));
    assert_eq!(last(Kind::Skill, "demo").as_deref(), Some("2026-11-01T10:00:00.000Z"), "newest of command + Skill tool");
    assert_eq!(last(Kind::Subagent, "reviewer").as_deref(), Some("2026-11-03T10:00:00.000Z"));
    assert_eq!(last(Kind::Mcp, "srv").as_deref(), Some("2026-11-04T10:00:00.000Z"));

    let plugin = find(&items, Kind::Plugin, "tool@mkt").unwrap();
    assert_eq!(plugin.last_used.as_deref(), Some("2026-11-05T10:00:00.000Z"), "plugin skill used from a subagent");
    assert!(plugin.notes.contains(&Note::Usage { used: 1 }));

    let unused = find(&items, Kind::Mcp, "local1").unwrap();
    assert!(unused.tracked && unused.last_used.is_none());
    assert!(items.iter().filter(|i| i.agent != "Claude Code").all(|i| !i.tracked), "only Claude Code has history");
}

#[test]
fn hook_only_plugins_are_not_called_unused() {
    let home = FakeHome::new();
    let install = |name: &str| serde_json::to_string(&home.path(&format!(".claude/plugins/cache/m/{name}/1.0.0"))).unwrap();
    home.write(
        ".claude/plugins/installed_plugins.json",
        &format!(
            r#"{{"plugins": {{"inline@m": [{{"installPath": {}}}], "listed@m": [{{"installPath": {}}}], "quiet@m": [{{"installPath": {}}}]}}}}"#,
            install("inline"),
            install("listed"),
            install("quiet")
        ),
    );
    // Hooks declared inline in plugin.json, as a list of paths, and a plugin with skills only.
    home.write(
        ".claude/plugins/cache/m/inline/1.0.0/.claude-plugin/plugin.json",
        r#"{"name": "inline", "hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "a"}]}]}}"#,
    );
    home.write(".claude/plugins/cache/m/listed/1.0.0/.claude-plugin/plugin.json", r#"{"name": "listed", "hooks": ["./hooks/extra.json"]}"#);
    home.write(".claude/plugins/cache/m/listed/1.0.0/hooks/extra.json", r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "b"}]}]}}"#);
    home.write(".claude/plugins/cache/m/quiet/1.0.0/skills/s/SKILL.md", "---\nname: s\ndescription: d\n---\n");

    let mut items = home.scan();
    crate::usage::Usage::read(&home.path(".claude/projects")).apply(&mut items);
    let notes = |name| find(&items, Kind::Plugin, name).unwrap().notes.clone();

    assert!(notes("inline@m").contains(&Note::Contents { skills: 0, agents: 0, commands: 0, hooks: 1 }));
    assert!(notes("listed@m").contains(&Note::Contents { skills: 0, agents: 0, commands: 0, hooks: 1 }));
    assert!(notes("inline@m").contains(&Note::RunsViaHooks));
    assert!(notes("listed@m").contains(&Note::RunsViaHooks));
    assert!(!notes("quiet@m").contains(&Note::RunsViaHooks), "skills only: really unused");
}
