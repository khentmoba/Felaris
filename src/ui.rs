pub const SIDEBAR_WIDTH_EXPANDED: f64 = 260.0;
pub const SIDEBAR_WIDTH_COLLAPSED: f64 = 68.0;

pub fn get_ui_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Felaris Sidebar</title>
<style>
  :root {
    --bg-window: #090b0e;
    --bg-sidebar: #101319;
    --bg-card: #171b23;
    --bg-card-hover: #1f2531;
    --bg-active: #232b38;
    --border-subtle: #1c222e;
    --border-focus: #38bdf8;
    --text-primary: #f8fafc;
    --text-secondary: #94a3b8;
    --text-muted: #64748b;
    --accent: #38bdf8;
    --accent-glow: rgba(56, 189, 248, 0.22);
    --sleeping: #a78bfa;
    --sleeping-bg: rgba(167, 139, 250, 0.12);
    --success: #22c55e;
    --gold: #facc15;
    --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  }

  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  body {
    background-color: var(--bg-sidebar);
    color: var(--text-primary);
    font-family: var(--font-sans);
    height: 100vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border-subtle);
  }

  /* --- Top Window / Nav Bar --- */
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 6px 12px;
    gap: 6px;
  }

  .nav-group {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .icon-btn {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .icon-btn:hover {
    background-color: var(--bg-card-hover);
    color: var(--text-primary);
  }

  .icon-btn:active {
    transform: scale(0.92);
  }

  .brand-badge {
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.8px;
    color: var(--accent);
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(56, 189, 248, 0.1);
  }

  /* --- Omnibox Pill --- */
  .omnibox-wrapper {
    padding: 6px 12px 8px 12px;
  }

  .omnibox {
    display: flex;
    align-items: center;
    height: 32px;
    background-color: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 0 8px;
    gap: 6px;
    transition: all 0.18s;
    position: relative;
    overflow: hidden;
  }

  .omnibox:focus-within {
    border-color: var(--border-focus);
    box-shadow: 0 0 0 2px var(--accent-glow);
    background-color: #1a202c;
  }

  .omnibox-icon {
    width: 14px;
    height: 14px;
    color: var(--text-muted);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .omnibox:focus-within .omnibox-icon {
    color: var(--accent);
  }

  .url-input {
    flex: 1;
    height: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 12.5px;
    font-family: inherit;
    user-select: text;
    -webkit-user-select: text;
  }

  .url-input::placeholder {
    color: var(--text-muted);
  }

  .copy-btn {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.12s;
  }

  .copy-btn:hover {
    color: var(--text-primary);
  }

  /* Progress Laser */
  .laser-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    height: 2px;
    width: 0%;
    background: linear-gradient(90deg, #38bdf8, #818cf8);
    opacity: 0;
    transition: width 0.3s ease, opacity 0.2s ease;
  }

  .laser-bar.loading {
    opacity: 1;
    width: 75%;
    animation: laserPulse 1.4s infinite ease-in-out;
  }

  .laser-bar.finished {
    width: 100%;
    opacity: 0;
    transition: width 0.15s ease, opacity 0.3s ease 0.15s;
  }

  @keyframes laserPulse {
    0% { filter: brightness(1); }
    50% { filter: brightness(1.5); }
    100% { filter: brightness(1); }
  }

  /* --- Pinned Essentials Dock (Arc 4-Grid) --- */
  .pinned-dock {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    padding: 4px 12px 10px 12px;
  }

  .dock-tile {
    height: 32px;
    border-radius: 8px;
    background-color: var(--bg-card);
    border: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    color: var(--text-secondary);
  }

  .dock-tile:hover {
    background-color: var(--bg-card-hover);
    color: var(--text-primary);
    transform: translateY(-1px);
    border-color: rgba(56, 189, 248, 0.3);
  }

  .dock-tile svg {
    width: 16px;
    height: 16px;
  }

  /* --- Spaces / Workspaces Bar --- */
  .spaces-bar {
    display: flex;
    align-items: center;
    padding: 0 12px 8px 12px;
    gap: 6px;
  }

  .space-chip {
    flex: 1;
    height: 26px;
    border-radius: 6px;
    background-color: transparent;
    border: 1px solid transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .space-chip:hover {
    background-color: var(--bg-card-hover);
    color: var(--text-primary);
  }

  .space-chip.active {
    background-color: var(--bg-card);
    color: var(--text-primary);
    border-color: var(--border-subtle);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
  }

  /* --- Tabs Section Header --- */
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px 4px 12px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.6px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .section-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .section-btn {
    font-size: 10px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px 5px;
    border-radius: 4px;
    transition: all 0.12s;
  }

  .section-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-card-hover);
  }

  /* --- Vertical Tabs List --- */
  .tabs-scroll-area {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .tabs-scroll-area::-webkit-scrollbar {
    width: 4px;
  }
  .tabs-scroll-area::-webkit-scrollbar-thumb {
    background-color: var(--border-subtle);
    border-radius: 2px;
  }

  .tab-card {
    display: flex;
    align-items: center;
    height: 34px;
    padding: 0 8px;
    border-radius: 8px;
    background-color: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.14s cubic-bezier(0.16, 1, 0.3, 1);
    gap: 8px;
    border: 1px solid transparent;
    position: relative;
  }

  .tab-card:hover {
    background-color: var(--bg-card-hover);
    color: var(--text-primary);
  }

  .tab-card.active {
    background-color: var(--bg-card);
    color: var(--text-primary);
    border-color: var(--border-subtle);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
  }

  .tab-card.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 3px;
    background-color: var(--accent);
    border-radius: 0 2px 2px 0;
  }

  .tab-favicon {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background-color: rgba(255, 255, 255, 0.08);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .tab-title-text {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
  }

  .tab-sleep-tag {
    font-size: 9.5px;
    padding: 1px 4px;
    border-radius: 3px;
    background-color: var(--sleeping-bg);
    color: var(--sleeping);
    font-weight: 600;
    flex-shrink: 0;
  }

  .tab-close-btn {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    color: var(--text-muted);
    opacity: 0;
    transition: all 0.12s;
    flex-shrink: 0;
  }

  .tab-card:hover .tab-close-btn {
    opacity: 1;
  }

  .tab-close-btn:hover {
    background-color: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  /* --- Footer & RAM Optimizer --- */
  .sidebar-footer {
    padding: 8px 12px 10px 12px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: var(--bg-sidebar);
  }

  .ram-saver-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 14px;
    background-color: var(--sleeping-bg);
    border: 1px solid rgba(167, 139, 250, 0.2);
    font-size: 11px;
    font-weight: 600;
    color: var(--sleeping);
    cursor: pointer;
    transition: all 0.15s;
  }

  .ram-saver-pill:hover {
    background-color: rgba(167, 139, 250, 0.22);
    box-shadow: 0 0 10px rgba(167, 139, 250, 0.35);
  }

  /* Collapsed Mode Styling */
  body.collapsed .sidebar-header .brand-badge,
  body.collapsed .omnibox-wrapper,
  body.collapsed .spaces-bar,
  body.collapsed .section-header,
  body.collapsed .tab-title-text,
  body.collapsed .tab-sleep-tag,
  body.collapsed .tab-close-btn,
  body.collapsed .ram-saver-pill span:not(:first-child) {
    display: none;
  }

  body.collapsed .pinned-dock {
    grid-template-columns: 1fr;
    gap: 4px;
    padding: 4px 8px;
  }

  body.collapsed .tab-card {
    justify-content: center;
    padding: 0;
  }

  body.collapsed .tab-card.active::before {
    left: 2px;
  }

  /* Toast */
  .toast {
    position: absolute;
    bottom: 45px;
    left: 12px;
    right: 12px;
    background-color: #1a202c;
    border: 1px solid var(--border-focus);
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    text-align: center;
    opacity: 0;
    transform: translateY(4px);
    transition: all 0.2s ease;
    pointer-events: none;
    z-index: 100;
  }

  .toast.show {
    opacity: 1;
    transform: translateY(0);
  }

  svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }
</style>
</head>
<body>

  <!-- Top Navigation Controls -->
  <div class="sidebar-header">
    <div class="nav-group">
      <button class="icon-btn" id="btn-back" title="Back (Alt+Left)">
        <svg viewBox="0 0 24 24"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
      </button>
      <button class="icon-btn" id="btn-forward" title="Forward (Alt+Right)">
        <svg viewBox="0 0 24 24"><path d="M12 4l-1.41 1.41L16.17 11H4v2h12.17l-5.58 5.59L12 20l8-8z"/></svg>
      </button>
      <button class="icon-btn" id="btn-reload" title="Reload (Ctrl+R / F5)">
        <svg viewBox="0 0 24 24"><path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
      </button>
      <button class="icon-btn" id="btn-home" title="Start Page">
        <svg viewBox="0 0 24 24"><path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/></svg>
      </button>
    </div>

    <button class="icon-btn" id="btn-toggle-sidebar" title="Collapse / Expand Sidebar (Ctrl+S)">
      <svg viewBox="0 0 24 24"><path d="M3 3h18v18H3V3zm6 2v14h12V5H9zm-4 0v14h2V5H5z"/></svg>
    </button>
  </div>

  <!-- Smart Omnibox -->
  <div class="omnibox-wrapper">
    <div class="omnibox">
      <div class="omnibox-icon" id="omnibox-icon">
        <svg viewBox="0 0 24 24"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
      </div>
      <input type="text" class="url-input" id="url-input" placeholder="Search or type URL (Ctrl+L)..." autocomplete="off" spellcheck="false">
      <button class="copy-btn" id="btn-copy-url" title="Copy Current URL">
        <svg viewBox="0 0 24 24"><path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/></svg>
      </button>
      <div class="laser-bar" id="laser-bar"></div>
    </div>
  </div>

  <!-- Pinned Essentials Dock -->
  <div class="pinned-dock">
    <div class="dock-tile" data-url="https://github.com" title="GitHub">
      <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.1-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85V21c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0 0 12 2z"/></svg>
    </div>
    <div class="dock-tile" data-url="https://doc.rust-lang.org/book/" title="Rust Docs">
      <svg viewBox="0 0 24 24"><path d="M18 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zM6 4h5v8l-2.5-1.5L6 12V4z"/></svg>
    </div>
    <div class="dock-tile" data-url="https://crates.io" title="Crates.io">
      <svg viewBox="0 0 24 24"><path d="M21 16.5c0 .38-.21.71-.53.88l-7.9 4.44c-.16.12-.36.18-.57.18s-.41-.06-.57-.18l-7.9-4.44A.991.991 0 013 16.5v-9c0-.38.21-.71.53-.88l7.9-4.44c.16-.12.36-.18.57-.18s.41.06.57.18l7.9 4.44c.32.17.53.5.53.88v9z"/></svg>
    </div>
    <div class="dock-tile" data-url="https://duckduckgo.com" title="DuckDuckGo">
      <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/></svg>
    </div>
  </div>

  <!-- Workspaces Chips -->
  <div class="spaces-bar" id="spaces-bar">
    <div class="space-chip active" data-space-id="1">
      <span>🪐</span>
      <span>General</span>
    </div>
    <div class="space-chip" data-space-id="2">
      <span>⚡</span>
      <span>Dev</span>
    </div>
    <div class="space-chip" data-space-id="3">
      <span>📖</span>
      <span>Read</span>
    </div>
  </div>

  <!-- Tabs Section -->
  <div class="section-header">
    <span>Open Tabs</span>
    <div class="section-actions">
      <button class="section-btn" id="btn-clear-tabs" title="Clear non-active tabs">Clear</button>
      <button class="section-btn" id="btn-add-tab" title="New Tab (Ctrl+T)">+ New</button>
    </div>
  </div>

  <!-- Vertical Scrollable Tabs -->
  <div class="tabs-scroll-area" id="tabs-container"></div>

  <!-- Footer -->
  <div class="sidebar-footer">
    <div class="ram-saver-pill" id="btn-ram-saver" title="Click to put background tabs to sleep">
      <span>⚡</span>
      <span id="ram-saved-label">Optimize RAM</span>
    </div>

    <button class="icon-btn" id="btn-reopen-tab" title="Restore Last Closed Tab (Ctrl+Shift+T)">
      <svg viewBox="0 0 24 24"><path d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z"/></svg>
    </button>
  </div>

  <div class="toast" id="toast">Feedback</div>

<script>
  let currentState = {
    tabs: [],
    active_tab_id: 1,
    active_workspace_id: 1,
    workspaces: [],
    sidebar_collapsed: false,
    is_loading: false
  };

  function sendIPC(action, payload = {}) {
    if (window.ipc) {
      window.ipc.postMessage(JSON.stringify({ action, ...payload }));
    }
  }

  function showToast(msg) {
    const t = document.getElementById('toast');
    t.textContent = msg;
    t.classList.add('show');
    setTimeout(() => t.classList.remove('show'), 1400);
  }

  function getDomainLetter(url) {
    try {
      if (url.startsWith('felaris://')) return '⚡';
      const parsed = new URL(url);
      const host = parsed.hostname.replace('www.', '');
      return host.charAt(0).toUpperCase();
    } catch {
      return '🌐';
    }
  }

  function renderSidebar() {
    if (currentState.sidebar_collapsed) {
      document.body.classList.add('collapsed');
    } else {
      document.body.classList.remove('collapsed');
    }

    // 1. Render Workspaces
    const spaceChips = document.querySelectorAll('.space-chip');
    spaceChips.forEach(chip => {
      const id = parseInt(chip.getAttribute('data-space-id'), 10);
      chip.classList.toggle('active', id === currentState.active_workspace_id);
    });

    // 2. Render Tabs
    const container = document.getElementById('tabs-container');
    container.innerHTML = '';

    let sleepingCount = 0;
    const currentWorkspaceTabs = currentState.tabs.filter(t => t.workspace_id === currentState.active_workspace_id);

    currentWorkspaceTabs.forEach(tab => {
      if (tab.is_sleeping) sleepingCount++;

      const tabEl = document.createElement('div');
      tabEl.className = 'tab-card' + (tab.id === currentState.active_tab_id ? ' active' : '');

      const badge = getDomainLetter(tab.url);
      let sleepHtml = tab.is_sleeping ? '<span class="tab-sleep-tag">💤 SLEEP</span>' : '';

      tabEl.innerHTML = `
        <div class="tab-favicon">${badge}</div>
        <div class="tab-title-text" title="${escapeHtml(tab.title || tab.url || 'New Tab')}">${escapeHtml(tab.title || tab.url || 'New Tab')}</div>
        ${sleepHtml}
        <div class="tab-close-btn" data-id="${tab.id}" title="Close Tab (Ctrl+W)">×</div>
      `;

      tabEl.addEventListener('click', (e) => {
        if (!e.target.classList.contains('tab-close-btn')) {
          sendIPC('switch_tab', { id: tab.id });
        }
      });

      tabEl.querySelector('.tab-close-btn').addEventListener('click', (e) => {
        e.stopPropagation();
        sendIPC('close_tab', { id: tab.id });
      });

      container.appendChild(tabEl);
    });

    // 3. Update RAM saved pill
    const saved = sleepingCount * 70;
    document.getElementById('ram-saved-label').textContent = saved > 0 ? `~${saved} MB Saved` : 'Optimize RAM';

    // 4. Update URL Input
    const activeTab = currentState.tabs.find(t => t.id === currentState.active_tab_id);
    const urlInput = document.getElementById('url-input');
    if (activeTab) {
      urlInput.value = (activeTab.url === 'felaris://newtab') ? '' : activeTab.url;
    }

    // 5. Laser Loading Line
    const laser = document.getElementById('laser-bar');
    if (currentState.is_loading) {
      laser.classList.add('loading');
      laser.classList.remove('finished');
    } else {
      laser.classList.add('finished');
      laser.classList.remove('loading');
    }
  }

  function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  // Update from Rust
  window.updateState = function(state) {
    currentState = state;
    renderSidebar();
  };

  // Nav Handlers
  document.getElementById('btn-back').addEventListener('click', () => sendIPC('back'));
  document.getElementById('btn-forward').addEventListener('click', () => sendIPC('forward'));
  document.getElementById('btn-reload').addEventListener('click', () => sendIPC('reload'));
  document.getElementById('btn-home').addEventListener('click', () => sendIPC('navigate', { url: 'felaris://newtab' }));
  document.getElementById('btn-toggle-sidebar').addEventListener('click', () => sendIPC('toggle_sidebar'));

  document.getElementById('btn-add-tab').addEventListener('click', () => sendIPC('new_tab'));
  document.getElementById('btn-clear-tabs').addEventListener('click', () => {
    sendIPC('clear_unpinned_tabs');
    showToast('Cleared non-active tabs');
  });

  document.getElementById('btn-ram-saver').addEventListener('click', () => {
    sendIPC('sleep_inactive_tabs');
    showToast('Background tabs put to sleep (RAM freed)');
  });

  document.getElementById('btn-reopen-tab').addEventListener('click', () => {
    sendIPC('restore_closed_tab');
  });

  document.getElementById('btn-copy-url').addEventListener('click', () => {
    const active = currentState.tabs.find(t => t.id === currentState.active_tab_id);
    if (active && active.url) {
      navigator.clipboard.writeText(active.url);
      showToast('URL Copied!');
    }
  });

  // Dock items
  document.querySelectorAll('.dock-tile').forEach(tile => {
    tile.addEventListener('click', () => {
      const url = tile.getAttribute('data-url');
      if (url) sendIPC('navigate', { url });
    });
  });

  // Workspace items
  document.querySelectorAll('.space-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      const id = parseInt(chip.getAttribute('data-space-id'), 10);
      sendIPC('switch_workspace', { id });
    });
  });

  // URL Input
  const urlInput = document.getElementById('url-input');
  urlInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      let val = urlInput.value.trim();
      if (!val) return;

      let targetUrl = val;
      if (val === 'felaris://newtab') {
        targetUrl = 'felaris://newtab';
      } else if (val.includes('.') && !val.includes(' ') && !val.startsWith('http://') && !val.startsWith('https://')) {
        targetUrl = 'https://' + val;
      } else if (!val.startsWith('http://') && !val.startsWith('https://') && !val.startsWith('file://')) {
        targetUrl = 'https://duckduckgo.com/?q=' + encodeURIComponent(val);
      }

      sendIPC('navigate', { url: targetUrl });
      urlInput.blur();
    }
  });

  // Keyboard Shortcuts
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key.toLowerCase() === 't') {
      e.preventDefault();
      sendIPC('new_tab');
    } else if (e.ctrlKey && e.key.toLowerCase() === 'w') {
      e.preventDefault();
      sendIPC('close_tab', { id: currentState.active_tab_id });
    } else if (e.ctrlKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      sendIPC('toggle_sidebar');
    } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 't') {
      e.preventDefault();
      sendIPC('restore_closed_tab');
    } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      sendIPC('sleep_inactive_tabs');
      showToast('Background tabs put to sleep');
    } else if (e.ctrlKey && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      urlInput.focus();
      urlInput.select();
    } else if (e.ctrlKey && e.key.toLowerCase() === 'r') {
      e.preventDefault();
      sendIPC('reload');
    } else if (e.altKey && e.key === '1') {
      sendIPC('switch_workspace', { id: 1 });
    } else if (e.altKey && e.key === '2') {
      sendIPC('switch_workspace', { id: 2 });
    } else if (e.altKey && e.key === '3') {
      sendIPC('switch_workspace', { id: 3 });
    }
  });
</script>
</body>
</html>
"#
}
