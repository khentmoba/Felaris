(() => {
  if (window.Felaris) return;
  function send(action, payload = {}) {
    window.ipc?.postMessage(JSON.stringify({ action, ...payload }));
  }
  function resolveAddress(value) {
    const text = value.trim();
    if (!text) return null;
    if (text === 'felaris://newtab') return text;
    if (/^(https?:\/\/|file:\/\/)/i.test(text)) return text;
    if (!/\s/.test(text) && /^(localhost(?=[:/]|$)|\[[0-9a-f:]+\](?=[:/]|$)|[^/:]+\.[^/:]+)(:\d+)?([/?#].*)?$/i.test(text)) {
      return (/^(localhost|127\.0\.0\.1|\[::1\])(?=[:/]|$)/i.test(text) ? 'http://' : 'https://') + text;
    }
    return window.Felaris.searchUrl + encodeURIComponent(text);
  }
  window.Felaris = { send, resolveAddress, navigate: url => send('navigate', { url }), searchUrl: 'https://search.brave.com/search?q=', searchName: 'Brave Search' };
  const postShortcut = window.ipc?.postMessage.bind(window.ipc);
  const stringify = JSON.stringify;
  window.addEventListener('keydown', event => {
    if (!event.isTrusted || event.defaultPrevented || event.isComposing || event.repeat) return;
    const key = event.key.toLowerCase();
    let action;
    let payload = {};
    if (event.ctrlKey && !event.altKey && !event.metaKey) {
      if (key === 't') action = event.shiftKey ? 'restore_closed_tab' : 'new_tab';
      else if (key === 's' && event.shiftKey) action = 'sleep_inactive_tabs';
      else if (key === 'b' && event.shiftKey) action = 'toggle_sidebar';
      else if (key === 'n' && event.shiftKey) action = 'new_private_tab';
      else if (key === 'd' && !event.shiftKey) action = 'toggle_bookmark';
      else if (key === 'w' && !event.shiftKey) action = 'close_active_tab';
      else if (key === 'l' && !event.shiftKey) action = 'focus_address';
      else if (key === 'j' && !event.shiftKey) action = 'open_downloads';
      else if (key === 'h' && !event.shiftKey) action = 'open_history';
      else if (key === 'r' && !event.shiftKey) action = 'reload';
      else if (key === 'tab') { action = 'cycle_tab'; payload = { backwards: event.shiftKey }; }
      else if (key === '=' || key === '+') { action = 'zoom'; payload = { delta: 10 }; }
      else if (key === '-' || key === '_') { action = 'zoom'; payload = { delta: -10 }; }
      else if (key === '0' && !event.shiftKey) { action = 'zoom'; payload = { delta: 0 }; }
    } else if (event.altKey && !event.ctrlKey && !event.shiftKey && !event.metaKey) {
      if (key === 'arrowleft') action = 'back';
      else if (key === 'arrowright') action = 'forward';
      else if (key === 'd') action = 'focus_address';
      else if (/^[1-9]$/.test(key)) { action = 'switch_profile'; payload = { index: Number(key) }; }
    } else if (!event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey) {
      if (key === 'f5') action = 'reload';
      else if (key === 'f11') action = 'toggle_fullscreen';
    }
    if (action) {
      event.preventDefault();
      postShortcut?.(stringify({ __proto__: null, action, ...payload, shortcut_token: '__SHORTCUT_TOKEN__' }));
    }
  }, true);
})();
