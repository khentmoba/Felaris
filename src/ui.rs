pub const UI_HEIGHT: f64 = 78.0;

pub fn get_ui_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Felaris UI</title>
<style>
  :root {
    --bg-primary: #121417;
    --bg-secondary: #1a1d22;
    --bg-hover: #262a32;
    --bg-active: #20242b;
    --border-color: #2b303a;
    --text-primary: #edf2f7;
    --text-muted: #8c96a5;
    --accent: #38bdf8;
    --accent-glow: rgba(56, 189, 248, 0.25);
    --sleeping: #a78bfa;
    --font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }

  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  body {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    font-family: var(--font-family);
    height: 78px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--border-color);
  }

  /* --- Top Tab Strip --- */
  .tab-strip {
    display: flex;
    align-items: center;
    height: 38px;
    padding: 4px 8px 0 8px;
    background-color: var(--bg-primary);
    overflow-x: auto;
    overflow-y: hidden;
  }

  .tab-strip::-webkit-scrollbar {
    display: none;
  }

  .tabs-container {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
  }

  .tab {
    display: flex;
    align-items: center;
    max-width: 190px;
    min-width: 110px;
    height: 32px;
    padding: 0 10px;
    border-radius: 8px 8px 0 0;
    background-color: var(--bg-secondary);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border-top: 2px solid transparent;
    position: relative;
  }

  .tab:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .tab.active {
    background-color: var(--bg-active);
    color: var(--text-primary);
    border-top: 2px solid var(--accent);
    box-shadow: 0 -2px 10px var(--accent-glow);
  }

  .tab-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-right: 6px;
  }

  .tab-close {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--text-muted);
    transition: background-color 0.1s;
  }

  .tab-close:hover {
    background-color: rgba(255, 255, 255, 0.15);
    color: #ff5555;
  }

  .tab-sleep-badge {
    font-size: 10px;
    color: var(--sleeping);
    margin-right: 4px;
    title: "Sleeping (RAM saved)";
  }

  .btn-new-tab {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 16px;
    margin-left: 4px;
    transition: background 0.15s;
  }

  .btn-new-tab:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  /* --- Bottom Nav / URL Bar --- */
  .nav-bar {
    display: flex;
    align-items: center;
    height: 40px;
    padding: 4px 10px;
    background-color: var(--bg-active);
    gap: 8px;
  }

  .nav-buttons {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .nav-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: transparent;
    border: none;
    color: var(--text-muted);
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
    transform: scale(0.95);
  }

  .url-bar-container {
    flex: 1;
    display: flex;
    align-items: center;
    position: relative;
  }

  .url-input {
    width: 100%;
    height: 30px;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 0 12px;
    color: var(--text-primary);
    font-size: 12.5px;
    outline: none;
    transition: all 0.15s;
    user-select: text;
    -webkit-user-select: text;
  }

  .url-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-glow);
    background-color: #16191f;
  }

  .browser-badge {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: var(--accent);
    padding: 0 8px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }
</style>
</head>
<body>

  <!-- Top Tab Strip -->
  <div class="tab-strip">
    <div class="tabs-container" id="tabs-container">
      <!-- Injected via JavaScript -->
    </div>
    <button class="btn-new-tab" id="btn-new-tab" title="New Tab (Ctrl+T)">+</button>
  </div>

  <!-- Bottom Nav & URL Bar -->
  <div class="nav-bar">
    <div class="nav-buttons">
      <button class="nav-btn" id="btn-back" title="Back (Alt+Left)">
        <svg viewBox="0 0 24 24"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
      </button>
      <button class="nav-btn" id="btn-forward" title="Forward (Alt+Right)">
        <svg viewBox="0 0 24 24"><path d="M12 4l-1.41 1.41L16.17 11H4v2h12.17l-5.58 5.59L12 20l8-8z"/></svg>
      </button>
      <button class="nav-btn" id="btn-reload" title="Reload (Ctrl+R / F5)">
        <svg viewBox="0 0 24 24"><path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
      </button>
      <button class="nav-btn" id="btn-home" title="Homepage">
        <svg viewBox="0 0 24 24"><path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/></svg>
      </button>
    </div>

    <div class="url-bar-container">
      <input type="text" class="url-input" id="url-input" placeholder="Search or enter web address..." autocomplete="off" spellcheck="false">
    </div>

    <div class="browser-badge" title="Felaris — Ultra-lean Rust Browser">
      <span>FELARIS</span>
    </div>
  </div>

<script>
  let currentState = {
    tabs: [],
    active_tab_id: 1,
    homepage: "https://duckduckgo.com"
  };

  function sendIPC(action, payload = {}) {
    if (window.ipc) {
      window.ipc.postMessage(JSON.stringify({ action, ...payload }));
    }
  }

  function renderTabs() {
    const container = document.getElementById('tabs-container');
    container.innerHTML = '';

    currentState.tabs.forEach(tab => {
      const tabEl = document.createElement('div');
      tabEl.className = 'tab' + (tab.id === currentState.active_tab_id ? ' active' : '');
      
      let titleHtml = '';
      if (tab.is_sleeping) {
        titleHtml += '<span class="tab-sleep-badge" title="Sleeping (RAM conserved)">💤</span>';
      }
      titleHtml += `<span class="tab-title">${escapeHtml(tab.title || tab.url || 'New Tab')}</span>`;

      tabEl.innerHTML = `
        ${titleHtml}
        <span class="tab-close" data-id="${tab.id}" title="Close Tab (Ctrl+W)">×</span>
      `;

      tabEl.addEventListener('click', (e) => {
        if (!e.target.classList.contains('tab-close')) {
          sendIPC('switch_tab', { id: tab.id });
        }
      });

      const closeBtn = tabEl.querySelector('.tab-close');
      closeBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        sendIPC('close_tab', { id: tab.id });
      });

      container.appendChild(tabEl);
    });

    // Update URL input if active tab exists
    const activeTab = currentState.tabs.find(t => t.id === currentState.active_tab_id);
    if (activeTab) {
      document.getElementById('url-input').value = activeTab.url || '';
    }
  }

  function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  // Called from Rust via evaluate_script
  window.updateState = function(state) {
    currentState = state;
    renderTabs();
  };

  // Nav actions
  document.getElementById('btn-new-tab').addEventListener('click', () => {
    sendIPC('new_tab');
  });

  document.getElementById('btn-back').addEventListener('click', () => {
    sendIPC('back');
  });

  document.getElementById('btn-forward').addEventListener('click', () => {
    sendIPC('forward');
  });

  document.getElementById('btn-reload').addEventListener('click', () => {
    sendIPC('reload');
  });

  document.getElementById('btn-home').addEventListener('click', () => {
    sendIPC('navigate', { url: currentState.homepage });
  });

  // URL Input
  const urlInput = document.getElementById('url-input');
  urlInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      let val = urlInput.value.trim();
      if (!val) return;

      // Smart URL formatting
      let targetUrl = val;
      if (val.includes('.') && !val.includes(' ') && !val.startsWith('http://') && !val.startsWith('https://')) {
        targetUrl = 'https://' + val;
      } else if (!val.startsWith('http://') && !val.startsWith('https://') && !val.startsWith('file://')) {
        targetUrl = 'https://duckduckgo.com/?q=' + encodeURIComponent(val);
      }

      sendIPC('navigate', { url: targetUrl });
      urlInput.blur();
    }
  });

  // Global Keyboard Shortcuts
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key.toLowerCase() === 't') {
      e.preventDefault();
      sendIPC('new_tab');
    } else if (e.ctrlKey && e.key.toLowerCase() === 'w') {
      e.preventDefault();
      sendIPC('close_tab', { id: currentState.active_tab_id });
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
