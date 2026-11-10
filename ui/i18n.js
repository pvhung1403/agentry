// English and Vietnamese UI text. Add a language by adding a dictionary with the same keys.
const dictionaries = {
  en: {
    tagline: 'What your AI agents load',
    library: 'Library',
    agents: 'Agents',
    all: 'All items',
    kinds: {
      skill: 'Skills', plugin: 'Plugins', memory: 'Memory', subagent: 'Subagents', command: 'Commands',
      mcp: 'MCP servers', hook: 'Hooks', marketplace: 'Marketplaces', leftover: 'Leftovers',
    },
    kind: {
      skill: 'Skill', plugin: 'Plugin', memory: 'Memory', subagent: 'Subagent', command: 'Command',
      mcp: 'MCP', hook: 'Hook', marketplace: 'Marketplace', leftover: 'Leftover',
    },
    search: 'Search name or description',
    refresh: 'Refresh',
    refreshHint: 'Scan again (⌘R / Ctrl+R)',
    remove: 'Remove',
    columns: { name: 'Name', agent: 'Agent', kind: 'Kind', bytes: 'Size', tokens: 'Tokens', lastUsed: 'Last used', status: 'Status' },
    unused30: 'Unused 30+ days',
    lastUsed: 'Last used',
    notUsed: 'Not used',
    notUsedSince: 'Not used since {date}',
    viaHooks: 'Via hooks',
    lastUsedHelp: 'From Claude Code session history since {date}. Other agents are not tracked yet.',
    items: 'items',
    onDisk: 'on disk',
    tokensPerSession: 'tokens loaded per session',
    tokensHelp: 'Skill, agent and command descriptions plus always-loaded memory, for enabled items. Estimated at 4 characters per token.',
    cleanUp: 'Clean up {size}',
    reclaimable: 'Reclaimable',
    unused: 'Unused',
    on: 'On',
    off: 'Off',
    scanning: 'Scanning your agents…',
    emptyTitle: 'Nothing found',
    emptyBody: 'No supported agent folders were found in your home folder.',
    noMatch: 'No results for “{q}”',
    noSelection: 'Select an item to see where it lives, what it costs and how it will be removed.',
    selectedN: '{n} items selected',
    removeN: 'Remove {n} items…',
    removeOne: 'Remove…',
    reveal: 'Show in folder',
    open: 'Open',
    size: 'Size',
    tokens: 'Tokens',
    path: 'Path',
    notesLabel: 'Notes',
    projectOnly: 'this project only',
    confirmOne: 'Remove {name}?',
    confirmMany: 'Remove {n} items?',
    cancel: 'Cancel',
    andMore: '…and {n} more',
    failed: 'Some actions failed',
    dismiss: 'Dismiss',
    removal: {
      trash: 'Moves {path} to the Trash.',
      cli: 'Runs `{cmd}`.',
      cliIn: 'Runs `{cmd}` in {cwd}.',
      jsonKey: 'Removes {key} from {file}. A backup is saved first.',
      manual: 'Edit {file} by hand to remove this.',
    },
    notes: {
      linkTo: 'Link to {target}',
      hooks: '{count} command(s) · {location}',
      projectScope: 'Only in project {project}',
      projectMemory: '{count} memory file(s) for this project · loads only there',
      unusedMarketplace: 'No installed plugin comes from here.',
      usage: '{used} of its skills, agents or MCP servers used in your history',
      runsViaHooks: "Works through hooks, which the history doesn't record, so it may be in use.",
      leftover: 'Old plugin version left in the cache. Claude Code no longer uses it.',
      restartAfterClean: 'Restart running Claude Code sessions after cleaning.',
      folderRefs: "Removing the folder can leave references in the agent's config.",
    },
    contents: { skills: 'skill', agents: 'agent', commands: 'command', hooks: 'hook' },
    errors: {
      toolNotFound: 'Could not find the `{tool}` command. Install it or add it to your PATH.',
      notJsonObject: 'The config file is not a JSON object, so it was not changed.',
      manual: 'This item has to be removed by hand.',
    },
  },
  vi: {
    tagline: 'Thứ AI agent nạp vào',
    library: 'Thư viện',
    agents: 'Agent',
    all: 'Tất cả',
    kinds: {
      skill: 'Skill', plugin: 'Plugin', memory: 'Memory', subagent: 'Subagent', command: 'Lệnh',
      mcp: 'MCP server', hook: 'Hook', marketplace: 'Marketplace', leftover: 'Rác còn sót',
    },
    kind: {
      skill: 'Skill', plugin: 'Plugin', memory: 'Memory', subagent: 'Subagent', command: 'Lệnh',
      mcp: 'MCP', hook: 'Hook', marketplace: 'Marketplace', leftover: 'Rác sót',
    },
    search: 'Tìm theo tên hoặc mô tả',
    refresh: 'Quét lại',
    refreshHint: 'Quét lại (⌘R / Ctrl+R)',
    remove: 'Xóa',
    columns: { name: 'Tên', agent: 'Agent', kind: 'Loại', bytes: 'Dung lượng', tokens: 'Token', lastUsed: 'Dùng lần cuối', status: 'Trạng thái' },
    unused30: 'Không dùng 30+ ngày',
    lastUsed: 'Dùng lần cuối',
    notUsed: 'Chưa dùng',
    notUsedSince: 'Chưa dùng từ {date}',
    viaHooks: 'Qua hook',
    lastUsedHelp: 'Theo lịch sử phiên Claude Code từ {date}. Chưa theo dõi các agent khác.',
    items: 'mục',
    onDisk: 'trên ổ đĩa',
    tokensPerSession: 'token nạp mỗi phiên',
    tokensHelp: 'Mô tả của skill, agent, lệnh và memory luôn được nạp, chỉ tính mục đang bật. Ước lượng 4 ký tự mỗi token.',
    cleanUp: 'Dọn {size}',
    reclaimable: 'Dọn được',
    unused: 'Không dùng',
    on: 'Bật',
    off: 'Tắt',
    scanning: 'Đang quét các agent…',
    emptyTitle: 'Không tìm thấy gì',
    emptyBody: 'Không thấy thư mục agent nào được hỗ trợ trong thư mục home.',
    noMatch: 'Không có kết quả cho “{q}”',
    noSelection: 'Chọn một mục để xem nó nằm ở đâu, tốn bao nhiêu và sẽ được xóa thế nào.',
    selectedN: 'Đã chọn {n} mục',
    removeN: 'Xóa {n} mục…',
    removeOne: 'Xóa…',
    reveal: 'Mở thư mục chứa',
    open: 'Mở',
    size: 'Dung lượng',
    tokens: 'Token',
    path: 'Đường dẫn',
    notesLabel: 'Ghi chú',
    projectOnly: 'chỉ trong dự án này',
    confirmOne: 'Xóa {name}?',
    confirmMany: 'Xóa {n} mục?',
    cancel: 'Hủy',
    andMore: '…và {n} mục khác',
    failed: 'Một số thao tác thất bại',
    dismiss: 'Đóng',
    removal: {
      trash: 'Chuyển {path} vào Thùng rác.',
      cli: 'Chạy `{cmd}`.',
      cliIn: 'Chạy `{cmd}` trong {cwd}.',
      jsonKey: 'Xóa {key} khỏi {file}. Bản sao lưu được lưu trước.',
      manual: 'Sửa {file} bằng tay để xóa mục này.',
    },
    notes: {
      linkTo: 'Liên kết tới {target}',
      hooks: '{count} lệnh · {location}',
      projectScope: 'Chỉ trong dự án {project}',
      projectMemory: '{count} file memory của dự án · chỉ nạp trong dự án đó',
      unusedMarketplace: 'Không có plugin nào đã cài đến từ đây.',
      usage: 'Đã dùng {used} skill, agent hoặc MCP của plugin trong lịch sử',
      runsViaHooks: 'Hoạt động qua hook, mà lịch sử không ghi lại, nên có thể vẫn đang được dùng.',
      leftover: 'Bản plugin cũ còn sót trong cache. Claude Code không còn dùng.',
      restartAfterClean: 'Khởi động lại các phiên Claude Code đang chạy sau khi dọn.',
      folderRefs: 'Xóa thư mục có thể để lại tham chiếu trong config của agent.',
    },
    contents: { skills: 'skill', agents: 'agent', commands: 'lệnh', hooks: 'hook' },
    errors: {
      toolNotFound: 'Không tìm thấy lệnh `{tool}`. Hãy cài nó hoặc thêm vào PATH.',
      notJsonObject: 'File config không phải JSON object nên không bị thay đổi.',
      manual: 'Mục này phải xóa bằng tay.',
    },
  },
};

const STORAGE_KEY = 'agentry.lang';
let current = detect();

function detect() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && dictionaries[saved]) return saved;
  } catch { /* storage can be unavailable */ }
  return (navigator.language || 'en').toLowerCase().startsWith('vi') ? 'vi' : 'en';
}

export function getLang() {
  return current;
}

export function setLang(lang) {
  if (!dictionaries[lang]) return;
  current = lang;
  try { localStorage.setItem(STORAGE_KEY, lang); } catch { /* ignore */ }
}

/** t('kinds.skill'), t('cleanUp', { size: '1 GB' }) */
export function t(key, params = {}) {
  const lookup = (dict) => key.split('.').reduce((node, part) => node?.[part], dict);
  const text = lookup(dictionaries[current]) ?? lookup(dictionaries.en) ?? key;
  return String(text).replace(/\{(\w+)\}/g, (_, name) => (name in params ? params[name] : `{${name}}`));
}

export function fmtBytes(bytes) {
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const exp = Math.min(Math.floor(Math.log10(bytes) / 3), units.length - 1);
  const value = bytes / 1000 ** exp;
  return `${value.toLocaleString(current, { maximumFractionDigits: value < 10 && exp > 0 ? 1 : 0 })} ${units[exp]}`;
}

export function fmtTokens(tokens) {
  return new Intl.NumberFormat(current, { notation: 'compact', maximumFractionDigits: 1 }).format(tokens);
}

/** "3 days ago" / "3 ngày trước" */
export function fmtRelative(iso) {
  const seconds = (Date.parse(iso) - Date.now()) / 1000;
  const units = [['year', 31_536_000], ['month', 2_592_000], ['week', 604_800], ['day', 86_400], ['hour', 3_600], ['minute', 60]];
  const format = new Intl.RelativeTimeFormat(current, { numeric: 'auto' });
  for (const [unit, size] of units) {
    if (Math.abs(seconds) >= size) return format.format(Math.round(seconds / size), unit);
  }
  return format.format(0, 'minute');
}

export function fmtDate(iso, withTime = false) {
  if (!iso) return '—';
  return new Date(iso).toLocaleString(current, withTime ? { dateStyle: 'medium', timeStyle: 'short' } : { dateStyle: 'medium' });
}

function plural(n, word) {
  return current === 'en' && n !== 1 ? `${n} ${word}s` : `${n} ${word}`;
}

/** Turns the backend's structured notes into localized lines. */
export function formatNotes(notes) {
  return notes.map((note) => {
    switch (note.code) {
      case 'version':
        return `v${note.value}`;
      case 'contents':
        return ['skills', 'agents', 'commands', 'hooks']
          .filter((k) => note[k] > 0)
          .map((k) => plural(note[k], t(`contents.${k}`)))
          .join(' · ');
      default:
        return t(`notes.${note.code}`, note);
    }
  }).filter(Boolean);
}

export function removalText(removal) {
  switch (removal.type) {
    case 'trash':
      return t('removal.trash', { path: removal.path });
    case 'cli':
      return removal.cwd
        ? t('removal.cliIn', { cmd: removal.argv.join(' '), cwd: removal.cwd })
        : t('removal.cli', { cmd: removal.argv.join(' ') });
    case 'jsonKey':
      return t('removal.jsonKey', { key: removal.keyPath.join(' › '), file: removal.file });
    default:
      return t('removal.manual', { file: removal.file });
  }
}

/** Backend errors start with a code when the UI should explain them. */
export function errorText(message) {
  const text = String(message);
  if (text.startsWith('tool-not-found:')) return t('errors.toolNotFound', { tool: text.slice('tool-not-found:'.length) });
  if (text === 'not-json-object') return t('errors.notJsonObject');
  if (text === 'manual') return t('errors.manual');
  return text;
}
