// Sample data for working on the UI in a plain browser (no Rust needed): `npx serve ui` and open the page.
// Shapes match the Rust `Item` type exactly; values are made up.
const H = '/Users/you';
const MB = 1_000_000;
const ago = (days) => new Date(Date.now() - days * 86_400_000).toISOString();
const historySince = ago(30);

function item(agent, kind, name, extra = {}) {
  const path = extra.path ?? null;
  // Claude Code history covers skills, subagents, commands, MCP servers and plugins.
  const tracked = agent === 'Claude Code' && ['skill', 'subagent', 'command', 'mcp', 'plugin'].includes(kind);
  return {
    id: `${agent}|${kind}|${path ?? ''}|${name}`,
    agent, kind, name,
    summary: '', path, bytes: 0, tokens: 0, perSession: true, enabled: null, toggle: null,
    removal: { type: 'trash', path },
    notes: [],
    tracked,
    lastUsed: null,
    ...extra,
  };
}

const cli = (...argv) => ({ type: 'cli', argv, cwd: null });
const claudePlugin = (id, extra) =>
  item('Claude Code', 'plugin', id, {
    enabled: true,
    toggle: { enable: ['claude', 'plugin', 'enable', id], disable: ['claude', 'plugin', 'disable', id] },
    removal: cli('claude', 'plugin', 'uninstall', id),
    ...extra,
  });

let items = [
  claudePlugin('devkit-pro@acme-tools', {
    lastUsed: ago(9), summary: 'Agents, skills and hooks for full-stack teams', path: `${H}/.claude/plugins/cache/acme-tools/devkit-pro/3.1.0`,
    bytes: 74 * MB, tokens: 33_400,
    notes: [{ code: 'version', value: '3.1.0' }, { code: 'contents', skills: 312, agents: 70, commands: 96, mcp: 1, hooks: 27 }, { code: 'usage', used: 3 }],
  }),
  claudePlugin('recall@memory-labs', {
    lastUsed: ago(0.1), summary: 'Persistent memory across sessions', path: `${H}/.claude/plugins/cache/memory-labs/recall/15.2.0`,
    bytes: 27 * MB, tokens: 2_060,
    notes: [{ code: 'version', value: '15.2.0' }, { code: 'contents', skills: 30, agents: 0, commands: 0, hooks: 11 }, { code: 'usage', used: 5 }],
  }),
  claudePlugin('codemap@graph-tools', {
    summary: 'Turn any codebase into an interactive knowledge graph', path: `${H}/.claude/plugins/cache/graph-tools/codemap/2.9.0`,
    bytes: 560 * MB, tokens: 830, enabled: false,
    notes: [{ code: 'version', value: '2.9.0' }, { code: 'contents', skills: 9, agents: 10, commands: 0, hooks: 0 }, { code: 'usage', used: 0 }],
  }),
  claudePlugin('terse@terse', {
    lastUsed: ago(1), summary: 'Shorter answers, fewer tokens', path: `${H}/.claude/plugins/cache/terse/terse/4.0.1`,
    bytes: 26 * MB, tokens: 1_320,
    notes: [{ code: 'version', value: '4.0.1' }, { code: 'contents', skills: 22, agents: 3, commands: 1, hooks: 0 }, { code: 'usage', used: 4 }],
  }),
  item('Claude Code', 'leftover', 'recall 15.0.0.tmp', {
    path: `${H}/.claude/plugins/cache/memory-labs/recall/15.0.0.tmp`, bytes: 896 * MB,
    notes: [{ code: 'leftover' }, { code: 'restartAfterClean' }],
  }),
  item('Claude Code', 'leftover', 'recall 14.8.0', {
    path: `${H}/.claude/plugins/cache/memory-labs/recall/14.8.0`, bytes: 519 * MB,
    notes: [{ code: 'leftover' }, { code: 'restartAfterClean' }],
  }),
  item('Claude Code', 'marketplace', 'old-experiments', {
    summary: 'someone/old-experiments', path: `${H}/.claude/plugins/marketplaces/old-experiments`, bytes: 41 * MB,
    removal: cli('claude', 'plugin', 'marketplace', 'remove', 'old-experiments'), notes: [{ code: 'unusedMarketplace' }],
  }),
  item('Claude Code', 'marketplace', 'acme-tools', {
    summary: 'acme/acme-tools', path: `${H}/.claude/plugins/marketplaces/acme-tools`, bytes: 108 * MB,
    removal: cli('claude', 'plugin', 'marketplace', 'remove', 'acme-tools'),
  }),
  item('Claude Code', 'skill', 'pdf-tools', {
    lastUsed: ago(6), summary: 'Read, merge and fill PDF files', path: `${H}/.claude/skills/pdf-tools`, bytes: 0.4 * MB, tokens: 38,
  }),
  item('Claude Code', 'skill', 'release-notes', {
    summary: 'Draft release notes from merged pull requests', path: `${H}/.claude/skills/release-notes`, bytes: 0.02 * MB, tokens: 24,
    notes: [{ code: 'linkTo', target: `${H}/.agents/skills/release-notes` }],
  }),
  item('Claude Code', 'memory', 'CLAUDE.md', {
    summary: 'Always answer in English unless asked otherwise', path: `${H}/.claude/CLAUDE.md`, bytes: 4_096, tokens: 410,
  }),
  item('Claude Code', 'memory', 'Projects-my-app', {
    path: `${H}/.claude/projects/-Users-you-Projects-my-app/memory`, bytes: 24_576, tokens: 180, perSession: false,
    notes: [{ code: 'projectMemory', count: 6 }],
  }),
  item('Claude Code', 'mcp', 'github', {
    lastUsed: ago(3), summary: 'https://api.githubcopilot.com/mcp/', path: `${H}/.claude.json`,
    removal: cli('claude', 'mcp', 'remove', 'github', '-s', 'user'),
  }),
  item('Claude Code', 'hook', 'PreToolUse', {
    summary: 'npx safety-check --strict', path: `${H}/.claude/settings.json`,
    removal: { type: 'jsonKey', file: `${H}/.claude/settings.json`, keyPath: ['hooks', 'PreToolUse'] },
    notes: [{ code: 'hooks', count: 1, location: 'hooks › PreToolUse' }],
  }),
  item('Claude Code', 'subagent', 'code-reviewer', {
    lastUsed: ago(12), summary: 'Reviews diffs for bugs and risky changes', path: `${H}/.claude/agents/code-reviewer.md`, bytes: 8_192, tokens: 22,
  }),
  item('Codex', 'memory', 'AGENTS.md', { summary: 'Prefer small, reviewed diffs', path: `${H}/.codex/AGENTS.md`, bytes: 2_048, tokens: 160 }),
  item('Codex', 'skill', 'sql-explainer', {
    summary: 'Explain slow SQL queries and suggest indexes', path: `${H}/.codex/skills/sql-explainer`, bytes: 0.05 * MB, tokens: 30,
  }),
  item('Codex', 'mcp', 'docs', { summary: `${H}/.codex/config.toml`, path: `${H}/.codex/config.toml`, removal: cli('codex', 'mcp', 'remove', 'docs') }),
  item('Cursor', 'mcp', 'linear', {
    summary: 'https://mcp.linear.app/sse', path: `${H}/.cursor/mcp.json`,
    removal: { type: 'jsonKey', file: `${H}/.cursor/mcp.json`, keyPath: ['mcpServers', 'linear'] },
  }),
  item('Cursor', 'hook', 'beforeSubmitPrompt', {
    summary: 'node ~/.cursor/hooks/redact.js', path: `${H}/.cursor/hooks.json`,
    removal: { type: 'jsonKey', file: `${H}/.cursor/hooks.json`, keyPath: ['hooks', 'beforeSubmitPrompt'] },
    notes: [{ code: 'hooks', count: 2, location: 'hooks › beforeSubmitPrompt' }],
  }),
  item('Cursor', 'subagent', 'test-writer', {
    summary: 'Writes focused unit tests', path: `${H}/.cursor/agents/test-writer.md`, bytes: 4_096, tokens: 18,
  }),
  item('Gemini CLI', 'memory', 'GEMINI.md', { summary: 'Project conventions', path: `${H}/.gemini/GEMINI.md`, bytes: 1_024, tokens: 95 }),
  item('Gemini CLI', 'plugin', 'conductor', {
    summary: 'Spec-driven development for Gemini CLI', path: `${H}/.gemini/extensions/conductor`, bytes: 3.2 * MB, tokens: 140,
    notes: [{ code: 'contents', skills: 0, agents: 0, commands: 6, hooks: 0 }, { code: 'folderRefs' }],
  }),
  item('Shared (~/.agents)', 'skill', 'release-notes', {
    summary: 'Draft release notes from merged pull requests', path: `${H}/.agents/skills/release-notes`, bytes: 0.06 * MB, tokens: 24,
  }),
];

const sampleFile = `---
name: pdf-tools
description: Read, merge and fill PDF files
---

# PDF tools

Use this skill when the user wants to read, merge, split or fill a PDF.
`;

// Parts of two sample plugins, for the inspector's "What's inside" list.
const partNames = {
  skill: ['api-design', 'backend-patterns', 'code-review', 'docker-patterns', 'e2e-testing', 'frontend-patterns', 'golang-patterns', 'python-testing', 'security-review', 'tdd-workflow'],
  subagent: ['architect', 'planner', 'build-error-resolver', 'refactor-cleaner'],
  command: ['plan', 'review', 'build-fix'],
  mcp: ['browser'],
};

function makeParts(counts, used) {
  const list = [];
  for (const [kind, n] of Object.entries(counts)) {
    const names = partNames[kind];
    for (let i = 0; i < n; i++) {
      const name = i < names.length ? names[i] : `${names[i % names.length]}-${Math.floor(i / names.length) + 1}`;
      const tokens = kind === 'mcp' ? 0 : 30 + ((i * 37) % 80);
      list.push({ kind, name, summary: '', tokens, tracked: true, lastUsed: used[name] ?? null });
    }
  }
  return list.sort((a, b) => b.tokens - a.tokens || a.name.localeCompare(b.name));
}

const mockParts = {
  [items[0].id]: makeParts({ skill: 312, subagent: 70, command: 96, mcp: 1 }, { 'code-review': ago(9), planner: ago(15), browser: ago(2) }),
  [items[1].id]: makeParts({ skill: 30 }, Object.fromEntries(partNames.skill.slice(0, 5).map((n, i) => [n, ago(i * 0.05)]))),
};

export const mockApi = {
  scan: async () => ({ items: structuredClone(items), historySince }),
  remove: async (ids) => {
    items = items.filter((i) => !ids.includes(i.id));
    return [];
  },
  setEnabled: async (id, enabled) => {
    items = items.map((i) => (i.id === id ? { ...i, enabled } : i));
  },
  reveal: async () => {},
  open: async () => {},
  components: async (id) => mockParts[id] ?? [],
  preview: async (id) => (items.find((i) => i.id === id)?.kind === 'skill' ? sampleFile : null),
};
