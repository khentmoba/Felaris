pub const UI_HEIGHT_NORMAL: f64 = 78.0;
pub const UI_HEIGHT_WITH_BOOKMARKS: f64 = 110.0;

pub fn get_ui_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Felaris Chrome</title>
<style>
  :root {
    --bg-base: #0c0e12;
    --bg-header: #12151b;
    --bg-surface: #181c24;
    --bg-hover: #222834;
    --bg-active: #202733;
    --border-subtle: #1e2430;
    --border-strong: #2d3545;
    --text-primary: #f8fafc;
    --text-secondary: #94a3b8;
    --text-muted: #64748b;
    --accent: #38bdf8;
    --accent-glow: rgba(56, 189, 248, 0.25);
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
    background-color: var(--bg-base);
    color: var(--text-primary);
    font-family: var(--font-sans);
    height: 100vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--border-subtle);
  }

  /* --- Top Tab Strip (Height: 38px) --- */
  .tab-strip {
    display: flex;
    align-items: center;
    height: 38px;
    padding: 4px 10px 0 10px;
    background-color: var(--bg-base);
    gap: 6px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .tabs-container {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .tabs-container::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: flex;
    align-items: center;
    max-width: 210px;
    min-width: 120px;
    height: 32px;
    padding: 0 10px;
    border-radius: 8px 8px 0 0;
    background-color: var(--bg-header);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    border: 1px solid transparent;
    border-bottom: none;
    position: relative;
    gap: 8px;
  }

  .tab:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .tab.active {
    background-color: var(--bg-surface);
    color: var(--text-primary);
    border-color: var(--border-strong);
    box-shadow: 0 -2px 8px var(--accent-glow);
  }

  .tab.active::after {
    content: '';
    position: absolute;
    top: 0;
    left: 8px;
    right: 8px;
    height: 2px;
    background: linear-gradient(90deg, #38bdf8, #818cf8);
    border-radius: 2px;
  }

  .tab-badge {
    width: 14px;
    height: 14px;
    border-radius: 4px;
    background-color: rgba(255, 255, 255, 0.08);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .tab-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
  }

  .tab-sleep-tag {
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 4px;
    background-color: var(--sleeping-bg);
    color: var(--sleeping);
    font-weight: 600;
    flex-shrink: 0;
  }

  .tab-close {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    color: var(--text-muted);
    transition: all 0.12s;
    opacity: 0.6;
    flex-shrink: 0;
  }

  .tab:hover .tab-close {
    opacity: 1;
  }

  .tab-close:hover {
    background-color: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .btn-strip {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 15px;
    transition: all 0.15s;
    flex-shrink: 0;
  }

  .btn-strip:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .strip-right {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }

  .memory-saver-pill {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    background-color: var(--sleeping-bg);
    border: 1px solid rgba(167, 139, 250, 0.2);
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
    color: var(--sleeping);
    cursor: pointer;
    transition: all 0.15s;
  }

  .memory-saver-pill:hover {
    background-color: rgba(167, 139, 250, 0.2);
    box-shadow: 0 0 10px rgba(167, 139, 250, 0.3);
  }

  /* --- Navigation & Omnibox Row (Height: 40px) --- */
  .nav-bar {
    display: flex;
    align-items: center;
    height: 40px;
    padding: 4px 10px;
    background-color: var(--bg-header);
    gap: 8px;
    position: relative;
  }

  .nav-controls {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .nav-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .nav-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-btn:active {
    transform: scale(0.94);
  }

  .nav-btn.active-toggle {
    color: var(--gold);
  }

  /* Smart Omnibox */
  .omnibox {
    flex: 1;
    display: flex;
    align-items: center;
    height: 30px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 0 8px;
    gap: 8px;
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
    overflow: hidden;
  }

  .omnibox:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-glow);
    background-color: #1a202a;
  }

  .ssl-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--success);
    font-size: 11px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .ssl-pill.search-mode {
    color: var(--accent);
  }

  .url-input {
    flex: 1;
    height: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    user-select: text;
    -webkit-user-select: text;
  }

  .url-input::placeholder {
    color: var(--text-muted);
  }

  .omnibox-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .action-btn {
    width: 22px;
    height: 22px;
    border-radius: 4px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s;
  }

  .action-btn:hover {
    color: var(--text-primary);
    background-color: rgba(255, 255, 255, 0.08);
  }

  .action-btn.bookmarked {
    color: var(--gold);
  }

  /* Progress Laser Bar */
  .progress-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    height: 2px;
    width: 0%;
    background: linear-gradient(90deg, #38bdf8, #818cf8, #a855f7);
    transition: width 0.3s ease, opacity 0.2s ease;
    opacity: 0;
    box-shadow: 0 0 8px rgba(56, 189, 248, 0.8);
  }

  .progress-bar.loading {
    opacity: 1;
    width: 75%;
    animation: progressPulse 1.5s infinite ease-in-out;
  }

  .progress-bar.finished {
    width: 100%;
    opacity: 0;
    transition: width 0.15s ease, opacity 0.3s ease 0.15s;
  }

  @keyframes progressPulse {
    0% { filter: brightness(1); }
    50% { filter: brightness(1.4); }
    100% { filter: brightness(1); }
  }

  /* Toast Notification */
  .toast {
    position: absolute;
    top: 44px;
    right: 20px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-strong);
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--accent);
    box-shadow: 0 4px 16px rgba(0,0,0,0.4);
    opacity: 0;
    transform: translateY(-4px);
    transition: all 0.2s ease;
    pointer-events: none;
    z-index: 100;
  }

  .toast.show {
    opacity: 1;
    transform: translateY(0);
  }

  /* --- Bookmarks Shelf (Height: 32px when visible) --- */
  .bookmarks-shelf {
    display: none;
    align-items: center;
    height: 32px;
    padding: 0 12px;
    background-color: var(--bg-surface);
    border-top: 1px solid var(--border-subtle);
    gap: 8px;
    overflow-x: auto;
  }

  .bookmarks-shelf.visible {
    display: flex;
  }

  .bookmark-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }

  .bookmark-pill:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .bookmark-icon {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }

  svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }
</style>
</head>
<body>

  <!-- 1. Top Tab Strip -->
  <div class="tab-strip">
    <div class="tabs-container" id="tabs-container"></div>
    <button class="btn-strip" id="btn-new-tab" title="New Tab (Ctrl+T)">+</button>

    <div class="strip-right">
      <div class="memory-saver-pill" id="btn-memory-saver" title="Click to put all inactive tabs to sleep and save RAM">
        <span>⚡</span>
        <span id="ram-saved-text">0 MB Saved</span>
      </div>
    </div>
  </div>

  <!-- 2. Navigation & Smart Omnibox -->
  <div class="nav-bar">
    <div class="nav-controls">
      <button class="nav-btn" id="btn-back" title="Back (Alt+Left)">
        <svg viewBox="0 0 24 24"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
      </button>
      <button class="nav-btn" id="btn-forward" title="Forward (Alt+Right)">
        <svg viewBox="0 0 24 24"><path d="M12 4l-1.41 1.41L16.17 11H4v2h12.17l-5.58 5.59L12 20l8-8z"/></svg>
      </button>
      <button class="nav-btn" id="btn-reload" title="Reload (Ctrl+R / F5)">
        <svg id="reload-icon" viewBox="0 0 24 24"><path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
      </button>
      <button class="nav-btn" id="btn-home" title="Felaris Start Page">
        <svg viewBox="0 0 24 24"><path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/></svg>
      </button>
    </div>

    <!-- Omnibox -->
    <div class="omnibox">
      <div class="ssl-pill" id="ssl-pill" title="Security Status">
        <svg id="ssl-icon" viewBox="0 0 24 24"><path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"/></svg>
      </div>

      <input type="text" class="url-input" id="url-input" placeholder="Search or type URL (Ctrl+L)..." autocomplete="off" spellcheck="false">

      <div class="omnibox-actions">
        <button class="action-btn" id="btn-copy-url" title="Copy Current URL">
          <svg viewBox="0 0 24 24"><path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/></svg>
        </button>
        <button class="action-btn" id="btn-bookmark" title="Bookmark This Page (Ctrl+D)">
          <svg viewBox="0 0 24 24"><path d="M12 17.27L18.18 21l-1.64-7.03L22 9.24l-7.19-.61L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21z"/></svg>
        </button>
        <button class="action-btn" id="btn-sleep-tab" title="Put Tab to Sleep (Frees RAM)">
          <svg viewBox="0 0 24 24"><path d="M12.3 2a10 10 0 0 0-1.9 20 10 10 0 0 0 9.6-7.3 1 1 0 0 0-1.3-1.2 8 8 0 1 1-7.6-10.2 1 1 0 0 0 1.2-1.3A10 10 0 0 0 12.3 2z"/></svg>
        </button>
        <button class="action-btn" id="btn-toggle-bookmarks" title="Toggle Bookmarks Shelf (Ctrl+B)">
          <svg viewBox="0 0 24 24"><path d="M19 18l2 1V3c0-1.1-.9-2-2-2H8.99C7.89 1 7 1.9 7 3h10c1.1 0 2 .9 2 2v13zM15 5H5c-1.1 0-2 .9-2 2v16l7-3 7 3V7c0-1.1-.9-2-2-2z"/></svg>
        </button>
      </div>

      <!-- Laser Loading Bar -->
      <div class="progress-bar" id="progress-bar"></div>
    </div>
  </div>

  <!-- 3. Bookmarks Shelf -->
  <div class="bookmarks-shelf" id="bookmarks-shelf">
    <div class="bookmark-pill" data-url="https://github.com">
      <svg class="bookmark-icon" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12c0 4.42 2.87 8.17 6.84 9.5.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34-.46-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.87 1.52 2.34 1.07 2.91.83.1-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.92 0-1.11.38-2 1.03-2.71-.1-.25-.45-1.29.1-2.64 0 0 .84-.27 2.75 1.02.79-.22 1.65-.33 2.5-.33.85 0 1.71.11 2.5.33 1.91-1.29 2.75-1.02 2.75-1.02.55 1.35.2 2.39.1 2.64.65.71 1.03 1.6 1.03 2.71 0 3.82-2.34 4.66-4.57 4.91.36.31.69.92.69 1.85V21c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0 0 12 2z"/></svg>
      <span>GitHub</span>
    </div>
    <div class="bookmark-pill" data-url="https://doc.rust-lang.org/book/">
      <svg class="bookmark-icon" viewBox="0 0 24 24"><path d="M18 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zM6 4h5v8l-2.5-1.5L6 12V4z"/></svg>
      <span>Rust Book</span>
    </div>
    <div class="bookmark-pill" data-url="https://crates.io">
      <svg class="bookmark-icon" viewBox="0 0 24 24"><path d="M21 16.5c0 .38-.21.71-.53.88l-7.9 4.44c-.16.12-.36.18-.57.18s-.41-.06-.57-.18l-7.9-4.44A.991.991 0 013 16.5v-9c0-.38.21-.71.53-.88l7.9-4.44c.16-.12.36-.18.57-.18s.41.06.57.18l7.9 4.44c.32.17.53.5.53.88v9z"/></svg>
      <span>Crates.io</span>
    </div>
    <div class="bookmark-pill" data-url="https://duckduckgo.com">
      <svg class="bookmark-icon" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/></svg>
      <span>DuckDuckGo</span>
    </div>
    <div class="bookmark-pill" data-url="https://youtube.com">
      <svg class="bookmark-icon" viewBox="0 0 24 24"><path d="M10 15l5.19-3L10 9v6m11.56-7.83c.13.47.22 1.1.28 1.9.07.8.1 1.49.1 2.09L22 12c0 2.19-.16 3.8-.44 4.83-.25.9-.83 1.48-1.73 1.73-.47.13-1.33.22-2.65.28-1.3.07-2.49.1-3.59.1L12 19c-4.19 0-6.8-.16-7.83-.44-.9-.25-1.48-.83-1.73-1.73-.13-.47-.22-1.1-.28-1.9-.07-.8-.1-1.49-.1-2.09L2 12c0-2.19.16-3.8.44-4.83.25-.9.83-1.48 1.73-1.73.47-.13 1.33-.22 2.65-.28 1.3-.07 2.49-.1 3.59-.1L12 5c4.19 0 6.8.16 7.83.44.9.25 1.48.83 1.73 1.73z"/></svg>
      <span>YouTube</span>
    </div>
    <div class="bookmark-pill" data-url="https://news.ycombinator.com">
      <svg class="bookmark-icon" viewBox="0 0 24 24"><path d="M3 3h18v18H3V3m9 10l3.5-7h-2.2L12 8.7 10.7 6H8.5l3.5 7v4h2v-4z"/></svg>
      <span>Hacker News</span>
    </div>
  </div>

  <div class="toast" id="toast">URL Copied!</div>

<script>
  let currentState = {
    tabs: [],
    active_tab_id: 1,
    homepage: "felaris://newtab",
    show_bookmarks: false,
    is_loading: false,
    ram_saved_mb: 0
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

  function getDomainBadge(url) {
    try {
      if (url.startsWith('felaris://')) return '⚡';
      const parsed = new URL(url);
      const host = parsed.hostname.replace('www.', '');
      return host.charAt(0).toUpperCase();
    } catch {
      return '🌐';
    }
  }

  function renderUI() {
    // 1. Render Tabs
    const container = document.getElementById('tabs-container');
    container.innerHTML = '';

    let sleepingCount = 0;

    currentState.tabs.forEach((tab, index) => {
      if (tab.is_sleeping) sleepingCount++;

      const tabEl = document.createElement('div');
      tabEl.className = 'tab' + (tab.id === currentState.active_tab_id ? ' active' : '');
      
      const badge = getDomainBadge(tab.url);
      let sleepTag = tab.is_sleeping ? '<span class="tab-sleep-tag">💤 SLEEP</span>' : '';

      tabEl.innerHTML = `
        <div class="tab-badge">${badge}</div>
        <div class="tab-title" title="${escapeHtml(tab.title || tab.url || 'New Tab')}">${escapeHtml(tab.title || tab.url || 'New Tab')}</div>
        ${sleepTag}
        <div class="tab-close" data-id="${tab.id}" title="Close Tab (Ctrl+W)">×</div>
      `;

      tabEl.addEventListener('click', (e) => {
        if (!e.target.classList.contains('tab-close')) {
          sendIPC('switch_tab', { id: tab.id });
        }
      });

      tabEl.querySelector('.tab-close').addEventListener('click', (e) => {
        e.stopPropagation();
        sendIPC('close_tab', { id: tab.id });
      });

      container.appendChild(tabEl);
    });

    // 2. RAM saved display
    const estimatedSaved = sleepingCount * 70;
    document.getElementById('ram-saved-text').textContent = estimatedSaved > 0 
      ? `~${estimatedSaved} MB Saved` 
      : 'Optimize RAM';

    // 3. Update Omnibox URL
    const activeTab = currentState.tabs.find(t => t.id === currentState.active_tab_id);
    const urlInput = document.getElementById('url-input');
    const sslIcon = document.getElementById('ssl-icon');
    const sslPill = document.getElementById('ssl-pill');

    if (activeTab) {
      if (activeTab.url === 'felaris://newtab') {
        urlInput.value = '';
        sslPill.classList.add('search-mode');
        sslIcon.innerHTML = '<path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>';
      } else {
        urlInput.value = activeTab.url;
        sslPill.classList.remove('search-mode');
        sslIcon.innerHTML = '<path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"/>';
      }
    }

    // 4. Progress bar state
    const progressBar = document.getElementById('progress-bar');
    if (currentState.is_loading) {
      progressBar.classList.add('loading');
      progressBar.classList.remove('finished');
    } else {
      progressBar.classList.add('finished');
      progressBar.classList.remove('loading');
    }

    // 5. Bookmarks shelf visibility
    const shelf = document.getElementById('bookmarks-shelf');
    const bookmarkToggleBtn = document.getElementById('btn-toggle-bookmarks');
    if (currentState.show_bookmarks) {
      shelf.classList.add('visible');
      bookmarkToggleBtn.classList.add('active-toggle');
    } else {
      shelf.classList.remove('visible');
      bookmarkToggleBtn.classList.remove('active-toggle');
    }
  }

  function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  // Called from Rust
  window.updateState = function(state) {
    currentState = state;
    renderUI();
  };

  // User Actions
  document.getElementById('btn-new-tab').addEventListener('click', () => sendIPC('new_tab'));
  document.getElementById('btn-back').addEventListener('click', () => sendIPC('back'));
  document.getElementById('btn-forward').addEventListener('click', () => sendIPC('forward'));
  document.getElementById('btn-reload').addEventListener('click', () => sendIPC('reload'));
  document.getElementById('btn-home').addEventListener('click', () => sendIPC('navigate', { url: 'felaris://newtab' }));

  document.getElementById('btn-memory-saver').addEventListener('click', () => {
    sendIPC('sleep_inactive_tabs');
    showToast('Inactive tabs put to sleep (RAM conserved)');
  });

  document.getElementById('btn-sleep-tab').addEventListener('click', () => {
    sendIPC('toggle_sleep', { id: currentState.active_tab_id });
    showToast('Toggled tab sleep state');
  });

  document.getElementById('btn-copy-url').addEventListener('click', () => {
    const activeTab = currentState.tabs.find(t => t.id === currentState.active_tab_id);
    if (activeTab && activeTab.url) {
      navigator.clipboard.writeText(activeTab.url);
      showToast('URL Copied to Clipboard!');
    }
  });

  document.getElementById('btn-toggle-bookmarks').addEventListener('click', () => {
    sendIPC('toggle_bookmarks_shelf');
  });

  document.getElementById('btn-bookmark').addEventListener('click', () => {
    const btn = document.getElementById('btn-bookmark');
    btn.classList.toggle('bookmarked');
    showToast(btn.classList.contains('bookmarked') ? 'Bookmarked!' : 'Bookmark removed');
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

  // Bookmarks Shelf Clicks
  document.querySelectorAll('.bookmark-pill').forEach(pill => {
    pill.addEventListener('click', () => {
      const url = pill.getAttribute('data-url');
      if (url) sendIPC('navigate', { url });
    });
  });

  // Keyboard Shortcuts
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key.toLowerCase() === 't') {
      e.preventDefault();
      sendIPC('new_tab');
    } else if (e.ctrlKey && e.key.toLowerCase() === 'w') {
      e.preventDefault();
      sendIPC('close_tab', { id: currentState.active_tab_id });
    } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 't') {
      e.preventDefault();
      sendIPC('restore_closed_tab');
    } else if (e.ctrlKey && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      sendIPC('toggle_bookmarks_shelf');
    } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      sendIPC('sleep_inactive_tabs');
      showToast('All background tabs put to sleep');
    } else if (e.ctrlKey && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      urlInput.focus();
      urlInput.select();
    } else if (e.ctrlKey && e.key.toLowerCase() === 'r') {
      e.preventDefault();
      sendIPC('reload');
    }
  });
</script>
</body>
</html>
"#
}
