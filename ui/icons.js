// 16px stroke icons, drawn in currentColor.
const paths = {
  grid: '<rect x="2" y="2" width="5" height="5" rx="1.2"/><rect x="9" y="2" width="5" height="5" rx="1.2"/><rect x="2" y="9" width="5" height="5" rx="1.2"/><rect x="9" y="9" width="5" height="5" rx="1.2"/>',
  skill: '<path d="M8 1.75 9.45 6.55 14.25 8 9.45 9.45 8 14.25 6.55 9.45 1.75 8 6.55 6.55Z"/>',
  plugin: '<path d="M8 1.5 14 4.75v6.5L8 14.5 2 11.25v-6.5Z"/><path d="M2 4.75 8 8l6-3.25M8 8v6.5"/>',
  memory: '<path d="M3 2.5h8.5A1.5 1.5 0 0 1 13 4v9.5H4.5A1.5 1.5 0 0 1 3 12Z"/><path d="M3 12a1.5 1.5 0 0 1 1.5-1.5H13M6 5.5h4"/>',
  subagent: '<circle cx="6" cy="5.5" r="2.25"/><path d="M1.75 13.5c.5-2.4 2.2-3.75 4.25-3.75s3.75 1.35 4.25 3.75"/><path d="M10.5 3.6a2.25 2.25 0 0 1 0 3.8M12 9.9c1.2.5 2 1.6 2.25 3.6"/>',
  command: '<rect x="1.75" y="2.75" width="12.5" height="10.5" rx="1.75"/><path d="m4.5 6.25 2 1.75-2 1.75M8.25 10h3.25"/>',
  mcp: '<rect x="2.25" y="2.25" width="11.5" height="4.75" rx="1.25"/><rect x="2.25" y="9" width="11.5" height="4.75" rx="1.25"/><path d="M5 4.6h.01M5 11.4h.01"/>',
  hook: '<path d="M6.75 9.25 9.25 6.75"/><path d="M7.25 4.75 8.4 3.6a2.6 2.6 0 0 1 3.7 3.7l-1.15 1.15M8.75 11.25 7.6 12.4a2.6 2.6 0 0 1-3.7-3.7l1.15-1.15"/>',
  marketplace: '<path d="M2.5 6.5v7h11v-7"/><path d="M1.75 6.5 3.25 2.5h9.5l1.5 4Z"/><path d="M6.25 13.5v-3.75h3.5v3.75"/>',
  leftover: '<path d="M2.5 4h11M6.25 4V2.5h3.5V4M3.75 4l.7 9.5h7.1l.7-9.5"/>',
  trash: '<path d="M2.5 4h11M6.25 4V2.5h3.5V4M3.75 4l.7 9.5h7.1l.7-9.5M6.5 7v4M9.5 7v4"/>',
  agent: '<rect x="4" y="4" width="8" height="8" rx="1.5"/><path d="M6.5 1.75V4M9.5 1.75V4M6.5 12v2.25M9.5 12v2.25M1.75 6.5H4M1.75 9.5H4M12 6.5h2.25M12 9.5h2.25"/>',
  search: '<circle cx="7" cy="7" r="4.5"/><path d="m10.5 10.5 3.25 3.25"/>',
  refresh: '<path d="M13.25 8a5.25 5.25 0 1 1-1.55-3.7"/><path d="M13.25 2.25v3h-3"/>',
  folder: '<path d="M1.75 4.25V12.5a1 1 0 0 0 1 1h10.5a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1H8L6.5 3.25H2.75a1 1 0 0 0-1 1Z"/>',
  external: '<path d="M9 2.5h4.5V7M13.5 2.5 7.25 8.75M11.5 9.5v3a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1h3"/>',
  sparkle: '<path d="M8 1.75 9.45 6.55 14.25 8 9.45 9.45 8 14.25 6.55 9.45 1.75 8 6.55 6.55Z"/>',
  inspect: '<rect x="1.75" y="2.25" width="12.5" height="11.5" rx="1.75"/><path d="M10 2.25v11.5"/>',
  close: '<path d="m4 4 8 8M12 4l-8 8"/>',
};

export function icon(name) {
  return `<svg class="icon" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] ?? paths.grid}</svg>`;
}
