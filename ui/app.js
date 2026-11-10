import { t, getLang, setLang, fmtBytes, fmtTokens, fmtRelative, fmtDate, formatNotes, removalText, errorText } from './i18n.js';
import { icon } from './icons.js';

// Inside the app, talk to the Rust backend. In a plain browser, use sample data so the UI can be developed without Rust.
const api = window.__TAURI__ ? tauriApi(window.__TAURI__.core.invoke) : (await import('./mock.js')).mockApi;

function tauriApi(invoke) {
  return {
    scan: () => invoke('scan'),
    remove: (ids) => invoke('remove', { ids }),
    setEnabled: (id, enabled) => invoke('set_enabled', { id, enabled }),
    reveal: (id) => invoke('reveal', { id }),
    open: (id) => invoke('open_item', { id }),
    preview: (id) => invoke('preview', { id }),
  };
}

const KINDS = ['skill', 'plugin', 'memory', 'subagent', 'command', 'mcp', 'hook', 'marketplace', 'leftover'];
const AGENT_ORDER = ['Claude Code', 'Codex', 'Cursor', 'Gemini CLI', 'Antigravity', 'Copilot CLI', 'Windsurf', 'Shared (~/.agents)'];
// The agent name is shown under each item's name, so it needs no column of its own.
const COLUMNS = [['name'], ['kind'], ['bytes', 'num'], ['tokens', 'num'], ['lastUsed'], ['status']];
const NUMERIC = ['bytes', 'tokens', 'lastUsed'];
const DAY = 86_400_000;
const STALE_DAYS = 30;

const state = {
  items: [],
  historySince: null,
  view: 'all', // 'all' | 'unused' | 'kind:<kind>' | 'agent:<name>'
  search: '',
  sort: { key: 'bytes', dir: -1 },
  selected: new Set(),
  anchor: null,
  busy: false,
  scanned: false,
  pending: [],
};

const $ = (selector) => document.querySelector(selector);
const esc = (value) =>
  String(value ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const sum = (list, pick) => list.reduce((total, item) => total + pick(item), 0);
const sessionTokens = (item) => (item.perSession && item.enabled !== false ? item.tokens : 0);
const canRemove = (item) => item.removal.type !== 'manual';
const selectedItems = () => state.items.filter((i) => state.selected.has(i.id));
// Hooks run without leaving a trace in the history, so a hook plugin with no recorded use is not "unused".
const viaHooks = (item) => !item.lastUsed && item.notes.some((n) => n.code === 'runsViaHooks');
const isStale = (item) =>
  item.tracked && !viaHooks(item) && (!item.lastUsed || Date.now() - Date.parse(item.lastUsed) > STALE_DAYS * DAY);
// Sort key: newest first, then never used, then items the history can't see.
const usageValue = (item) => (item.tracked && !viaHooks(item) ? (item.lastUsed ? Date.parse(item.lastUsed) : 0) : -1);
const lastUsedText = (item) =>
  !item.tracked ? '—' : item.lastUsed ? fmtRelative(item.lastUsed) : viaHooks(item) ? t('viaHooks') : t('notUsed');

// MARK: derived data

function inView(item) {
  if (state.view === 'all') return true;
  if (state.view === 'unused') return isStale(item);
  const split = state.view.indexOf(':');
  const type = state.view.slice(0, split);
  const value = state.view.slice(split + 1);
  return type === 'kind' ? item.kind === value : item.agent === value;
}

function visible() {
  const query = state.search.trim().toLowerCase();
  const { key, dir } = state.sort;
  const value = (item) => (key === 'kind' ? t(`kind.${item.kind}`) : key === 'lastUsed' ? usageValue(item) : item[key]);
  return state.items
    .filter((i) => inView(i) && (!query || i.name.toLowerCase().includes(query) || i.summary.toLowerCase().includes(query)))
    .sort((a, b) => {
      const x = value(a);
      const y = value(b);
      return (typeof x === 'number' ? x - y : String(x).localeCompare(String(y), getLang())) * dir;
    });
}

// MARK: rendering

function render() {
  const list = visible();
  renderSidebar();
  renderToolbar();
  renderSummary(list);
  renderTable(list);
  renderInspector();
}

function applyStaticText() {
  document.documentElement.lang = getLang();
  document.querySelectorAll('[data-i18n]').forEach((el) => (el.textContent = t(el.dataset.i18n)));
  document.querySelectorAll('[data-i18n-placeholder]').forEach((el) => (el.placeholder = t(el.dataset.i18nPlaceholder)));
  document.querySelectorAll('[data-i18n-title]').forEach((el) => (el.title = t(el.dataset.i18nTitle)));
  document.querySelectorAll('[data-lang]').forEach((el) => el.classList.toggle('active', el.dataset.lang === getLang()));
}

function renderSidebar() {
  const row = (view, label, iconName, count) =>
    `<button type="button" class="nav-item ${state.view === view ? 'active' : ''}" data-view="${esc(view)}">` +
    `${icon(iconName)}<span class="label">${esc(label)}</span><span class="count">${count}</span></button>`;
  const kinds = KINDS.map((k) => [k, state.items.filter((i) => i.kind === k).length]).filter(([, n]) => n > 0);
  const known = AGENT_ORDER.filter((a) => state.items.some((i) => i.agent === a));
  const others = [...new Set(state.items.map((i) => i.agent))].filter((a) => !AGENT_ORDER.includes(a));
  const agents = [...known, ...others];
  $('#nav').innerHTML = [
    `<div class="nav-section">${esc(t('library'))}</div>`,
    row('all', t('all'), 'grid', state.items.length),
    state.items.some((i) => i.tracked) ? row('unused', t('unused30'), 'clock', state.items.filter(isStale).length) : '',
    ...kinds.map(([k, n]) => row(`kind:${k}`, t(`kinds.${k}`), k, n)),
    agents.length ? `<div class="nav-section">${esc(t('agents'))}</div>` : '',
    ...agents.map((a) => row(`agent:${a}`, a, 'agent', state.items.filter((i) => i.agent === a).length)),
  ].join('');
}

function renderToolbar() {
  const refresh = $('#refresh');
  refresh.disabled = state.busy;
  refresh.classList.toggle('spin', state.busy);
  $('#remove').disabled = state.busy || !selectedItems().some(canRemove);
}

function renderSummary(list) {
  const leftovers = state.items.filter((i) => i.kind === 'leftover');
  const stat = (value, label, title = '') =>
    `<div class="stat" ${title ? `title="${esc(title)}"` : ''}><div class="value">${esc(value)}</div><div class="label">${esc(label)}</div></div>`;
  $('#summary').innerHTML = [
    stat(list.length.toLocaleString(getLang()), t('items')),
    stat(fmtBytes(sum(list, (i) => i.bytes)), t('onDisk')),
    stat(`~${fmtTokens(sum(list, sessionTokens))}`, t('tokensPerSession'), t('tokensHelp')),
    '<div class="spacer"></div>',
    leftovers.length
      ? `<button type="button" class="btn btn-warn" data-action="clean" ${state.busy ? 'disabled' : ''}>${icon('sparkle')}${esc(
          t('cleanUp', { size: fmtBytes(sum(leftovers, (i) => i.bytes)) }),
        )}</button>`
      : '',
  ].join('');
}

function renderTable(list) {
  $('#thead').innerHTML = `<tr>${COLUMNS.map(([key, cls = '']) => {
    const sortable = key !== 'status';
    const arrow = state.sort.key === key ? (state.sort.dir > 0 ? '▲' : '▼') : '';
    const help = key === 'lastUsed' && state.historySince ? ` title="${esc(t('lastUsedHelp', { date: fmtDate(state.historySince) }))}"` : '';
    return `<th class="${cls} ${sortable ? 'sortable' : ''}" ${sortable ? `data-sort="${key}"` : ''}${help}>${esc(t(`columns.${key}`))}<span class="arrow">${arrow}</span></th>`;
  }).join('')}</tr>`;
  $('#tbody').innerHTML = list.map(rowHTML).join('');

  const empty = $('#empty');
  let message = '';
  if (!state.scanned) message = `<p>${esc(t('scanning'))}</p>`;
  else if (!state.items.length) message = `<h3>${esc(t('emptyTitle'))}</h3><p>${esc(t('emptyBody'))}</p>`;
  else if (!list.length) message = `<p>${esc(t('noMatch', { q: state.search }))}</p>`;
  empty.innerHTML = message;
  empty.hidden = !message;
}

function rowHTML(item) {
  const agent = state.view.startsWith('agent:') ? '' : `<span class="agent-tag">${esc(item.agent)}</span>`;
  const sub = [agent, esc(item.summary)].filter(Boolean).join(' · ');
  const usageClass = !item.tracked || viaHooks(item) ? 'muted' : isStale(item) ? 'stale' : '';
  return `<tr data-id="${esc(item.id)}" class="${state.selected.has(item.id) ? 'selected' : ''}">
    <td><div class="name-cell"><span class="kind-icon k-${item.kind}">${icon(item.kind)}</span><div class="name-text">
      <div class="title">${esc(item.name)}</div>${sub ? `<div class="sub">${sub}</div>` : ''}</div></div></td>
    <td><span class="chip">${esc(t(`kind.${item.kind}`))}</span></td>
    <td class="num">${item.bytes ? fmtBytes(item.bytes) : '—'}</td>
    <td class="num ${sessionTokens(item) ? '' : 'muted'}">${item.tokens ? fmtTokens(item.tokens) : '—'}</td>
    <td class="${usageClass}">${esc(lastUsedText(item))}</td>
    <td>${statusHTML(item)}</td>
  </tr>`;
}

function statusHTML(item) {
  if (typeof item.enabled === 'boolean') {
    return `<label class="switch" title="${esc(t(item.enabled ? 'on' : 'off'))}"><input type="checkbox" data-toggle="${esc(item.id)}"
      ${item.enabled ? 'checked' : ''} ${state.busy ? 'disabled' : ''} aria-label="${esc(item.name)}"><span></span></label>`;
  }
  if (item.kind === 'leftover') return `<span class="badge warn">${esc(t('reclaimable'))}</span>`;
  if (item.notes.some((n) => n.code === 'unusedMarketplace')) return `<span class="badge">${esc(t('unused'))}</span>`;
  return '';
}

function lastUsedDetail(item) {
  if (item.lastUsed) return `${fmtRelative(item.lastUsed)} · ${fmtDate(item.lastUsed, true)}`;
  if (viaHooks(item)) return t('viaHooks');
  return state.historySince ? t('notUsedSince', { date: fmtDate(state.historySince) }) : t('notUsed');
}

let previewRequest = 0;

function renderInspector() {
  const el = $('#inspector');
  const items = selectedItems();
  if (!items.length) {
    el.innerHTML = `<div class="placeholder">${icon('inspect')}<p>${esc(t('noSelection'))}</p></div>`;
    return;
  }
  if (items.length > 1) {
    el.innerHTML = `<div class="placeholder"><h3>${esc(t('selectedN', { n: items.length }))}</h3>
      <p>${esc(fmtBytes(sum(items, (i) => i.bytes)))} ${esc(t('onDisk'))}</p>
      <button type="button" class="btn btn-danger" data-action="remove-selected" ${items.some(canRemove) ? '' : 'disabled'}>
        ${icon('trash')}${esc(t('removeN', { n: items.length }))}</button></div>`;
    return;
  }

  const item = items[0];
  const notes = formatNotes(item.notes);
  const tokens = item.tokens ? `~${fmtTokens(item.tokens)}${item.perSession ? '' : ` (${t('projectOnly')})`}` : '—';
  el.innerHTML = `
    <div class="insp-head"><span class="kind-icon big k-${item.kind}">${icon(item.kind)}</span>
      <div><h2>${esc(item.name)}</h2><div class="muted">${esc(item.agent)} · ${esc(t(`kind.${item.kind}`))}</div></div></div>
    ${item.summary ? `<p class="summary-text">${esc(item.summary)}</p>` : ''}
    <dl class="facts">
      <dt>${esc(t('size'))}</dt><dd>${item.bytes ? esc(fmtBytes(item.bytes)) : '—'}</dd>
      <dt>${esc(t('tokens'))}</dt><dd>${esc(tokens)}</dd>
      ${item.tracked ? `<dt>${esc(t('lastUsed'))}</dt><dd class="${isStale(item) ? 'stale' : ''}">${esc(lastUsedDetail(item))}</dd>` : ''}
      ${item.path ? `<dt>${esc(t('path'))}</dt><dd class="mono">${esc(item.path)}</dd>` : ''}
      ${notes.length ? `<dt>${esc(t('notesLabel'))}</dt><dd>${notes.map(esc).join('<br>')}</dd>` : ''}
    </dl>
    ${item.path ? `<div class="row-buttons">
      <button type="button" class="btn" data-action="reveal">${icon('folder')}${esc(t('reveal'))}</button>
      <button type="button" class="btn" data-action="open">${icon('external')}${esc(t('open'))}</button></div>` : ''}
    <div class="danger-zone">
      <button type="button" class="btn btn-danger" data-action="remove-selected" ${canRemove(item) && !state.busy ? '' : 'disabled'}>
        ${icon('trash')}${esc(t('removeOne'))}</button>
      <p class="muted small">${esc(removalText(item.removal))}</p>
    </div>
    <pre class="preview" id="preview" hidden></pre>`;

  const request = ++previewRequest;
  api.preview(item.id).then((text) => {
    const pre = $('#preview');
    if (text && pre && request === previewRequest) {
      pre.textContent = text;
      pre.hidden = false;
    }
  }).catch(() => {});
}

// MARK: actions

async function refresh() {
  state.busy = true;
  renderToolbar();
  try {
    const result = await api.scan();
    state.items = result.items;
    state.historySince = result.historySince ?? null;
  } catch (error) {
    showFailures([{ name: '', message: String(error) }]);
  }
  const ids = new Set(state.items.map((i) => i.id));
  state.selected = new Set([...state.selected].filter((id) => ids.has(id)));
  state.scanned = true;
  state.busy = false;
  render();
}

async function run(task) {
  state.busy = true;
  render();
  try {
    await task();
  } catch (error) {
    showFailures([{ name: '', message: String(error) }]);
  }
  await refresh();
}

function confirmRemoval(items) {
  const targets = items.filter(canRemove);
  if (!targets.length || state.busy) return;
  state.pending = targets;
  $('#confirm-title').textContent =
    targets.length === 1 ? t('confirmOne', { name: targets[0].name }) : t('confirmMany', { n: targets.length });
  $('#confirm-list').innerHTML =
    targets.slice(0, 8).map((i) => `<li><strong>${esc(i.name)}</strong><span>${esc(removalText(i.removal))}</span></li>`).join('') +
    (targets.length > 8 ? `<li class="muted">${esc(t('andMore', { n: targets.length - 8 }))}</li>` : '');
  $('#confirm').showModal();
}

function showFailures(failures) {
  const toast = $('#toast');
  toast.innerHTML = `<h4>${esc(t('failed'))}<button type="button" data-action="dismiss" aria-label="${esc(t('dismiss'))}">${icon('close')}</button></h4>
    <ul>${failures.map((f) => `<li>${f.name ? `<strong>${esc(f.name)}</strong>: ` : ''}${esc(errorText(f.message))}</li>`).join('')}</ul>`;
  toast.hidden = false;
}

function select(id, event) {
  const ids = visible().map((i) => i.id);
  if (event.shiftKey && state.anchor && ids.includes(state.anchor)) {
    const [from, to] = [ids.indexOf(state.anchor), ids.indexOf(id)].sort((a, b) => a - b);
    state.selected = new Set(ids.slice(from, to + 1));
  } else if (event.metaKey || event.ctrlKey) {
    if (state.selected.has(id)) state.selected.delete(id);
    else state.selected.add(id);
    state.anchor = id;
  } else {
    state.selected = new Set([id]);
    state.anchor = id;
  }
  render();
}

function moveSelection(step) {
  const ids = visible().map((i) => i.id);
  if (!ids.length) return;
  const current = ids.indexOf(state.anchor);
  const next = ids[Math.max(0, Math.min(ids.length - 1, current < 0 ? 0 : current + step))];
  state.selected = new Set([next]);
  state.anchor = next;
  render();
  document.querySelector(`tr[data-id="${CSS.escape(next)}"]`)?.scrollIntoView({ block: 'nearest' });
}

// MARK: events

$('#nav').addEventListener('click', (event) => {
  const button = event.target.closest('[data-view]');
  if (!button) return;
  state.view = button.dataset.view;
  state.selected.clear();
  render();
});

$('#thead').addEventListener('click', (event) => {
  const th = event.target.closest('[data-sort]');
  if (!th) return;
  const key = th.dataset.sort;
  state.sort = state.sort.key === key ? { key, dir: -state.sort.dir } : { key, dir: NUMERIC.includes(key) ? -1 : 1 };
  render();
});

$('#tbody').addEventListener('click', (event) => {
  if (event.target.closest('.switch')) return;
  const row = event.target.closest('tr[data-id]');
  if (row) select(row.dataset.id, event);
});

$('#tbody').addEventListener('dblclick', (event) => {
  const row = event.target.closest('tr[data-id]');
  if (row && !event.target.closest('.switch')) api.reveal(row.dataset.id).catch((e) => showFailures([{ name: '', message: e }]));
});

$('#tbody').addEventListener('change', (event) => {
  const id = event.target.dataset.toggle;
  if (id) run(() => api.setEnabled(id, event.target.checked));
});

$('#search').addEventListener('input', (event) => {
  state.search = event.target.value;
  render();
});

$('#refresh').addEventListener('click', refresh);

document.querySelectorAll('[data-lang]').forEach((button) =>
  button.addEventListener('click', () => {
    setLang(button.dataset.lang);
    applyStaticText();
    render();
  }),
);

document.addEventListener('click', (event) => {
  const action = event.target.closest('[data-action]')?.dataset.action;
  const [item] = selectedItems();
  if (action === 'remove-selected') confirmRemoval(selectedItems());
  else if (action === 'clean') confirmRemoval(state.items.filter((i) => i.kind === 'leftover'));
  else if (action === 'reveal' && item) api.reveal(item.id).catch((e) => showFailures([{ name: item.name, message: e }]));
  else if (action === 'open' && item) api.open(item.id).catch((e) => showFailures([{ name: item.name, message: e }]));
  else if (action === 'dismiss') $('#toast').hidden = true;
});

$('#confirm-cancel').addEventListener('click', () => $('#confirm').close());
$('#confirm-ok').addEventListener('click', () => {
  $('#confirm').close();
  const ids = state.pending.map((i) => i.id);
  state.pending = [];
  ids.forEach((id) => state.selected.delete(id));
  run(async () => {
    const failures = await api.remove(ids);
    if (failures.length) showFailures(failures);
  });
});

document.addEventListener('keydown', (event) => {
  const mod = event.metaKey || event.ctrlKey;
  const key = event.key.toLowerCase();
  const typing = event.target.matches('input[type="search"], textarea');
  if ($('#confirm').open) return;
  if (mod && key === 'r') {
    event.preventDefault();
    refresh();
  } else if (mod && key === 'f') {
    event.preventDefault();
    $('#search').focus();
  } else if (typing) {
    if (key === 'escape') event.target.blur();
  } else if (mod && key === 'a') {
    event.preventDefault();
    state.selected = new Set(visible().map((i) => i.id));
    render();
  } else if ((key === 'delete' || key === 'backspace') && state.selected.size) {
    event.preventDefault();
    confirmRemoval(selectedItems());
  } else if (key === 'arrowdown' || key === 'arrowup') {
    event.preventDefault();
    moveSelection(key === 'arrowdown' ? 1 : -1);
  } else if (key === 'escape') {
    state.selected.clear();
    render();
  }
});

// MARK: start

if (navigator.userAgent.includes('Mac')) document.body.classList.add('mac');
document.querySelectorAll('[data-icon]').forEach((el) => (el.outerHTML = icon(el.dataset.icon)));
applyStaticText();
render();
refresh();
