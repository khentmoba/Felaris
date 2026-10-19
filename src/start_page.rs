pub fn get_start_page_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Felaris — New Tab</title>
<style>
  :root {
    --bg-page: #0b0d10;
    --bg-surface: #13171f;
    --bg-surface-hover: #1b212c;
    --border-subtle: #1f2633;
    --border-focus: #38bdf8;
    --text-primary: #f1f5f9;
    --text-secondary: #94a3b8;
    --text-muted: #64748b;
    --accent: #38bdf8;
    --accent-glow: rgba(56, 189, 248, 0.2);
    --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  }

  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  body {
    background-color: var(--bg-page);
    background-image: 
      radial-gradient(circle at 50% 15%, rgba(56, 189, 248, 0.08) 0%, transparent 45%),
      radial-gradient(circle at 85% 85%, rgba(168, 85, 247, 0.05) 0%, transparent 40%);
    color: var(--text-primary);
    font-family: var(--font-sans);
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 24px;
    user-select: none;
    -webkit-user-select: none;
  }

  .container {
    width: 100%;
    max-width: 720px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 32px;
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Logo & Title */
  .brand-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .logo-mark {
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: linear-gradient(135deg, #0ea5e9, #6366f1);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 8px 24px -4px rgba(56, 189, 248, 0.35);
  }

  .logo-mark svg {
    width: 30px;
    height: 30px;
    fill: #ffffff;
  }

  .brand-title {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.5px;
    color: var(--text-primary);
  }

  .brand-subtitle {
    font-size: 14px;
    color: var(--text-secondary);
    font-weight: 400;
  }

  /* Search Box */
  .search-wrapper {
    width: 100%;
    max-width: 580px;
    position: relative;
  }

  .search-input {
    width: 100%;
    height: 48px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 24px;
    padding: 0 52px 0 20px;
    font-size: 15px;
    color: var(--text-primary);
    outline: none;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    user-select: text;
    -webkit-user-select: text;
  }

  .search-input:focus {
    border-color: var(--accent);
    box-shadow: 0 6px 28px var(--accent-glow), 0 0 0 2px var(--accent-glow);
    background-color: #161b24;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .search-btn {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: none;
    background-color: var(--accent);
    color: #0b0d10;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .search-btn:hover {
    background-color: #7dd3fc;
    transform: translateY(-50%) scale(1.05);
  }

  .search-btn svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
  }

  /* Quick Dials Grid */
  .quick-dials-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 16px;
    width: 100%;
    max-width: 580px;
  }

  .dial-card {
    background-color: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 14px;
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    text-decoration: none;
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .dial-card:hover {
    background-color: var(--bg-surface-hover);
    border-color: rgba(56, 189, 248, 0.4);
    transform: translateY(-3px);
    box-shadow: 0 8px 24px -4px rgba(0, 0, 0, 0.4);
  }

  .dial-icon {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: #1a202c;
  }

  .dial-icon svg {
    width: 20px;
    height: 20px;
    fill: var(--text-primary);
  }

  .dial-label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: color 0.15s;
  }

  .dial-card:hover .dial-label {
    color: var(--text-primary);
  }

  /* Performance Footer Banner */
  .perf-banner {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 20px;
    background-color: rgba(26, 32, 44, 0.5);
    border: 1px solid var(--border-subtle);
    border-radius: 20px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .perf-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: #38bdf8;
    font-weight: 600;
  }

  .perf-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: #22c55e;
    box-shadow: 0 0 8px #22c55e;
  }
</style>
</head>
<body>

<div class="container">
  <!-- Brand -->
  <div class="brand-header">
    <div class="logo-mark">
      <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/></svg>
    </div>
    <h1 class="brand-title">Felaris</h1>
    <p class="brand-subtitle">Smooth, ultra-lightweight browser crafted in Rust</p>
  </div>

  <!-- Search Bar -->
  <div class="search-wrapper">
    <input type="text" class="search-input" id="search-input" placeholder="Search the web or enter address..." autofocus autocomplete="off" spellcheck="false">
    <button class="search-btn" id="search-btn" title="Search">
      <svg viewBox="0 0 24 24"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
    </button>
  </div>

  <!-- Quick Dials -->
  <div class="quick-dials-grid">
    <div class="dial-card" data-url="https://github.com">
      <div class="dial-icon">
        <svg viewBox="0 0 24 24"><path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"/></svg>
      </div>
      <span class="dial-label">GitHub</span>
    </div>

    <div class="dial-card" data-url="https://doc.rust-lang.org/book/">
      <div class="dial-icon" style="color: #f97316;">
        <svg viewBox="0 0 24 24"><path d="M18 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zM6 4h5v8l-2.5-1.5L6 12V4z"/></svg>
      </div>
      <span class="dial-label">Rust Docs</span>
    </div>

    <div class="dial-card" data-url="https://crates.io">
      <div class="dial-icon" style="color: #eab308;">
        <svg viewBox="0 0 24 24"><path d="M21 16.5c0 .38-.21.71-.53.88l-7.9 4.44c-.16.12-.36.18-.57.18s-.41-.06-.57-.18l-7.9-4.44A.991.991 0 013 16.5v-9c0-.38.21-.71.53-.88l7.9-4.44c.16-.12.36-.18.57-.18s.41.06.57.18l7.9 4.44c.32.17.53.5.53.88v9zM12 4.15L6.04 7.5 12 10.85l5.96-3.35L12 4.15z"/></svg>
      </div>
      <span class="dial-label">Crates.io</span>
    </div>

    <div class="dial-card" data-url="https://duckduckgo.com">
      <div class="dial-icon" style="color: #f97316;">
        <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/></svg>
      </div>
      <span class="dial-label">DuckDuckGo</span>
    </div>

    <div class="dial-card" data-url="https://youtube.com">
      <div class="dial-icon" style="color: #ef4444;">
        <svg viewBox="0 0 24 24"><path d="M10 15l5.19-3L10 9v6m11.56-7.83c.13.47.22 1.1.28 1.9.07.8.1 1.49.1 2.09L22 12c0 2.19-.16 3.8-.44 4.83-.25.9-.83 1.48-1.73 1.73-.47.13-1.33.22-2.65.28-1.3.07-2.49.1-3.59.1L12 19c-4.19 0-6.8-.16-7.83-.44-.9-.25-1.48-.83-1.73-1.73-.13-.47-.22-1.1-.28-1.9-.07-.8-.1-1.49-.1-2.09L2 12c0-2.19.16-3.8.44-4.83.25-.9.83-1.48 1.73-1.73.47-.13 1.33-.22 2.65-.28 1.3-.07 2.49-.1 3.59-.1L12 5c4.19 0 6.8.16 7.83.44.9.25 1.48.83 1.73 1.73z"/></svg>
      </div>
      <span class="dial-label">YouTube</span>
    </div>

    <div class="dial-card" data-url="https://news.ycombinator.com">
      <div class="dial-icon" style="color: #f97316;">
        <svg viewBox="0 0 24 24"><path d="M3 3h18v18H3V3m9 10l3.5-7h-2.2L12 8.7 10.7 6H8.5l3.5 7v4h2v-4z"/></svg>
      </div>
      <span class="dial-label">Hacker News</span>
    </div>

    <div class="dial-card" data-url="https://wikipedia.org">
      <div class="dial-icon">
        <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/></svg>
      </div>
      <span class="dial-label">Wikipedia</span>
    </div>

    <div class="dial-card" data-url="https://reddit.com">
      <div class="dial-icon" style="color: #ff4500;">
        <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm6.2 8.8c.2 0 .4.1.5.3.3.4.3.9 0 1.3-.1.2-.3.3-.5.3-.2 0-.4-.1-.5-.2-.8.5-1.8.8-2.9.8l.5-2.3 1.6.4c.1.4.5.7 1 .7.6 0 1-.4 1-1s-.4-1-1-1c-.5 0-.9.3-1 .8l-1.9-.4c-.1 0-.2.1-.2.2l-.6 2.8c-1.2 0-2.3-.3-3.2-.8-.1.1-.3.2-.5.2-.2 0-.4-.1-.5-.3-.3-.4-.3-.9 0-1.3.1-.2.3-.3.5-.3.1 0 .3 0 .4.1.7-.8 1.8-1.3 3-1.4l.6-2.9c0-.1.2-.2.3-.2h.1l2.2.5c.2-.4.6-.7 1.1-.7.6 0 1.1.5 1.1 1.1s-.5 1.1-1.1 1.1z"/></svg>
      </div>
      <span class="dial-label">Reddit</span>
    </div>
  </div>

  <!-- Performance Badge -->
  <div class="perf-banner">
    <span class="perf-pill"><span class="perf-dot"></span> 0.0% Idle CPU</span>
    <span>•</span>
    <span>Zero background telemetry</span>
    <span>•</span>
    <span style="color: #a78bfa;">Smart Tab Sleeping Active</span>
  </div>
</div>

<script>
  function navigate(url) {
    if (window.ipc) {
      window.ipc.postMessage(JSON.stringify({ action: "navigate", url: url }));
    } else {
      window.location.href = url;
    }
  }

  const searchInput = document.getElementById('search-input');
  const searchBtn = document.getElementById('search-btn');

  function doSearch() {
    const val = searchInput.value.trim();
    if (!val) return;

    let targetUrl = val;
    if (val.includes('.') && !val.includes(' ') && !val.startsWith('http://') && !val.startsWith('https://')) {
      targetUrl = 'https://' + val;
    } else if (!val.startsWith('http://') && !val.startsWith('https://') && !val.startsWith('file://')) {
      targetUrl = 'https://duckduckgo.com/?q=' + encodeURIComponent(val);
    }
    navigate(targetUrl);
  }

  searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') doSearch();
  });
  searchBtn.addEventListener('click', doSearch);

  document.querySelectorAll('.dial-card').forEach(card => {
    card.addEventListener('click', () => {
      const url = card.getAttribute('data-url');
      if (url) navigate(url);
    });
  });
</script>
</body>
</html>
"#
}
