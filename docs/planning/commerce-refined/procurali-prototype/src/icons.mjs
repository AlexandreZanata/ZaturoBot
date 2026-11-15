/**
 * Procurali — Original inline line icons (24px grid, 1.9px stroke, round caps) & Brand Mark.
 * Decorative by default (aria-hidden); pair every icon with a visible text label.
 */

const PATHS = {
  home: '<path d="M3.5 10.5 12 4l8.5 6.5"/><path d="M5.5 9.5V20h13V9.5"/><path d="M10 20v-5.5h4V20"/>',
  tag: '<path d="M3.5 12.2V4.5a1 1 0 0 1 1-1h7.7a1 1 0 0 1 .7.3l7.6 7.6a1 1 0 0 1 0 1.4l-7.7 7.7a1 1 0 0 1-1.4 0l-7.6-7.6a1 1 0 0 1-.3-.7Z"/><circle cx="8.2" cy="8.2" r="1.5"/>',
  ticket: '<path d="M3.5 8.5A1.5 1.5 0 0 1 5 7h14a1.5 1.5 0 0 1 1.5 1.5v2a1.5 1.5 0 0 0 0 3v2A1.5 1.5 0 0 1 19 17H5a1.5 1.5 0 0 1-1.5-1.5v-2a1.5 1.5 0 0 0 0-3v-2Z"/><path d="M10 7v10"/><path d="M14.5 10.5h.01M14.5 13.5h.01"/>',
  book: '<path d="M4 5.5A1.5 1.5 0 0 1 5.5 4H11v15.5H5.5A1.5 1.5 0 0 0 4 21Z"/><path d="M20 5.5A1.5 1.5 0 0 0 18.5 4H13v15.5h5.5A1.5 1.5 0 0 1 20 21Z"/>',
  sparkle: '<path d="M12 3.5 13.9 9l5.6 1.9-5.6 1.9L12 18.5l-1.9-5.7L4.5 10.9 10.1 9Z"/><path d="M19 3v3M17.5 4.5h3"/>',
  users: '<circle cx="9" cy="8.5" r="3.1"/><path d="M3.5 19.5c.6-3.2 2.8-5 5.5-5s4.9 1.8 5.5 5"/><path d="M15.5 5.6a3 3 0 0 1 0 5.8M17.5 14.8c1.6.6 2.7 2.2 3 4.7"/>',
  zap: '<path d="M13 2.8 5 13.4h6.2L10.3 21.2 19 10.4h-6.1Z"/>',
  bolt: '<path d="M13 2.8 5 13.4h6.2L10.3 21.2 19 10.4h-6.1Z"/>',
  bookmark: '<path d="M6.5 3.5h11a1 1 0 0 1 1 1v16l-6.5-4.2-6.5 4.2v-16a1 1 0 0 1 1-1Z"/>',
  search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m15.5 15.5 5 5"/>',
  menu: '<path d="M4 7h16M4 12h16M4 17h16"/>',
  close: '<path d="m6 6 12 12M18 6 6 18"/>',
  arrowRight: '<path d="M4.5 12h15M13.5 6l6 6-6 6"/>',
  chevronRight: '<path d="m9 6 6 6-6 6"/>',
  external: '<path d="M18 13v5.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H11"/><path d="M15 4h5v5"/><path d="M10 14 20 4"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  clock: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
  shield: '<path d="M12 3.5 19 6v5.5c0 4.4-3 7.7-7 9-4-1.3-7-4.6-7-9V6Z"/><path d="m9 12 2.2 2.2L15.5 10"/>',
  coins: '<ellipse cx="10" cy="8" rx="6" ry="2.8"/><path d="M4 8v4c0 1.5 2.7 2.8 6 2.8s6-1.3 6-2.8V8"/><path d="M4 12v4c0 1.5 2.7 2.8 6 2.8 1.3 0 2.6-.2 3.6-.6"/><path d="M14.5 12.5c3 0 5.5 1.1 5.5 2.5v3.5c0 1.4-2.5 2.5-5.5 2.5S9 19.9 9 18.5V16"/>',
  link: '<path d="M10 13.5a4.5 4.5 0 0 0 6.4 0l2.4-2.4a4.5 4.5 0 0 0-6.4-6.4l-1.4 1.3"/><path d="M14 10.5a4.5 4.5 0 0 0-6.4 0l-2.4 2.4a4.5 4.5 0 0 0 6.4 6.4l1.3-1.3"/>',
  heart: '<path d="M12 20s-7-4.4-8.8-8.9A5.2 5.2 0 0 1 12 5.6a5.2 5.2 0 0 1 8.8 5.5C19 15.6 12 20 12 20Z"/>',
  flame: '<path d="M12 3c.5 3.2 4 5.2 4 9.2a4 4 0 1 1-8 0c0-1.6.6-3 1.5-4.2.2 1.3 1.1 2.2 2 2.3-.4-2.2.2-5 1.5-7.3Z"/>',
  crown: '<path d="M3.5 17.5h17L19 7.5l-4 3.8L12 6l-3 5.3L5 7.5Z"/><path d="M5.5 20h13"/>',
  bell: '<path d="M18 15.5H6c1.2-1.3 1.8-2.8 1.8-5a4.2 4.2 0 1 1 8.4 0c0 2.2.6 3.7 1.8 5Z"/><path d="M10.2 18.5a2 2 0 0 0 3.6 0"/>',
  percent: '<line x1="18.5" y1="5.5" x2="5.5" y2="18.5"/><circle cx="7.5" cy="7.5" r="2.2"/><circle cx="16.5" cy="16.5" r="2.2"/>',
  scale: '<path d="M12 4v16M7 20h10M5 7.5h14"/><path d="m5 7.5-2.5 6a3 3 0 0 0 5 0Zm14 0-2.5 6a3 3 0 0 0 5 0Z"/>',
  info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.8v.2"/>',
  send: '<path d="M21 4 3 11.2l6.2 2.4 2.4 6.2Z"/><path d="m9.2 13.6 5.2-5.2"/>',
  message: '<path d="M4.5 18.5 5.8 15A7.5 7.5 0 1 1 9.5 18.5l-5 1Z"/>',
  monitor: '<rect x="3.5" y="4" width="17" height="11" rx="1.5"/><path d="M12 15v5M8 20h8"/>',
  laptop: '<rect x="4.5" y="5" width="15" height="10" rx="1.5"/><path d="M2.5 18h19"/>',
  lamp: '<path d="M4 6.5h16"/><path d="M6 6.5 7.5 10h9L18 6.5"/><path d="M9.5 13.5 8 15.5M12 14v2.5M14.5 13.5l1.5 2"/>',
  grid: '<rect x="4" y="4" width="7" height="7" rx="1.2"/><rect x="13" y="4" width="7" height="7" rx="1.2"/><rect x="4" y="13" width="7" height="7" rx="1.2"/><rect x="13" y="13" width="7" height="7" rx="1.2"/>',
  calendar: '<rect x="4" y="5" width="16" height="15" rx="1.5"/><path d="M4 9.5h16M8.5 3.5v3M15.5 3.5v3"/>',
  plug: '<path d="M9 4v4M15 4v4M7 8h10v3.5a5 5 0 0 1-10 0Z"/><path d="M12 16.5V20"/>',
  store: '<path d="M4 9.5 5.5 4.5h13L20 9.5"/><path d="M3.5 9.5a2.5 2.5 0 0 0 5 0 2.5 2.5 0 0 0 5 0 2.5 2.5 0 0 0 5 0"/><path d="M5.5 11.8V19.5h13v-7.7"/><path d="M10 19.5V15h4v4.5"/>',
  whatsapp: '<path d="M12 20a8 8 0 1 0-6.8-3.8L4 20l4-.9A8 8 0 0 0 12 20Z"/><path d="M9.4 9.2c.2-.4.4-.4.6-.4h.5c.2 0 .4.1.5.4l.6 1.4c.1.2 0 .4-.1.6l-.4.4c.5 1 1.3 1.7 2.2 2.2l.4-.4c.2-.1.4-.2.6-.1l1.4.6c.3.1.4.3.4.5v.5c0 .3-.2.7-.6.9-.6.3-1.6.3-2.9-.3-1.4-.6-2.7-1.9-3.3-3.3-.6-1.3-.6-2.3-.3-2.9Z"/>',
};

export const CATEGORY_ICONS = {
  "ergonomia-e-suportes": "laptop",
  "energia-e-cabos": "plug",
  "iluminacao-de-bancada": "lamp",
  "organizacao-de-mesa": "grid",
};

export function icon(name, className = "icon") {
  const body = PATHS[name] || PATHS.info;
  return `<svg class="${className}" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${body}</svg>`;
}

export const BRAND_SYMBOL_SVG = `<svg class="brand-symbol" viewBox="0 0 64 64" width="32" height="32" aria-hidden="true" focusable="false">
  <path fill-rule="evenodd" fill="#72C82C" d="M8.5 28.5 L22.5 13.5 C24.2 11.7 26.6 10.7 29.1 10.7 H51.5 C55.6 10.7 59 14.1 59 18.2 V43.8 C59 47.9 55.6 51.3 51.5 51.3 H29.1 C26.6 51.3 24.2 50.3 22.5 48.5 L8.5 33.5 Q6.6 31 8.5 28.5 Z M23.5 26.3 A4.7 4.7 0 1 0 23.5 35.7 A4.7 4.7 0 1 0 23.5 26.3 Z" transform="rotate(-18 33 31)"/>
</svg>`;
