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
  function suggestions(value,state={},historyEntries=[]) {
    const query=value.trim();
    const needle=query.toLowerCase();
    const target=Felaris.resolveAddress(query);
    const options=[];
    const seen=new Set();
    const norm=u=>(u||'').replace(/\/+$/,'').toLowerCase();
    if(target) {
     const search=target===Felaris.searchUrl+encodeURIComponent(query);
     options.push({title:query,detail:search?'Search with '+Felaris.searchName.replace(/ Search$/,''):'Open in a new tab',url:target,search});
     seen.add(norm(target));
    }
    for(const tab of (state.tabs||[]).filter(tab=>!['felaris://newtab','felaris://blank'].includes(tab.url)&&(!needle||(tab.title+' '+tab.url).toLowerCase().includes(needle))).slice(0,6)) {
     const workspace=(state.workspaces||[]).find(space=>space.id===tab.workspace_id);
     options.push({title:tab.title||tab.url,detail:`Switch to tab${workspace?' · '+workspace.name:''} · ${tab.url}`,id:tab.id,url:tab.url,favicon:tab.favicon});
     seen.add(norm(tab.url));
    }
    for(const entry of (state.bookmarks||[])) {
     if(options.length>=8) break;
     if(!entry?.url||seen.has(norm(entry.url))) continue;
     if(needle&&!((entry.title||'')+' '+entry.url).toLowerCase().includes(needle)) continue;
     seen.add(norm(entry.url));
     options.push({title:entry.title||entry.url,detail:`Bookmark · ${entry.url}`,url:entry.url,bookmark:true});
    }
    for(const entry of historyEntries) {
     if(options.length>=8) break;
     if(!entry?.url||seen.has(norm(entry.url))) continue;
     if(needle&&!((entry.title||'')+' '+entry.url).toLowerCase().includes(needle)) continue;
     seen.add(norm(entry.url));
     options.push({title:entry.title||entry.url,detail:`History · ${entry.url}`,url:entry.url,history:true});
    }
    for(const item of (state.shortcuts||[])) {
     if(options.length>=8) break;
     if(!item?.url||seen.has(norm(item.url))) continue;
     if(needle&&!((item.title||'')+' '+item.url).toLowerCase().includes(needle)) continue;
     seen.add(norm(item.url));
     options.push({title:item.title||item.url,detail:`Shortcut · ${item.url}`,url:item.url,favicon:item.favicon,shortcut:true});
    }
    return options;
  }
  window.Felaris = { send, resolveAddress, suggestions, navigate: url => send('navigate', { url }), searchUrl: 'https://search.brave.com/search?q=', searchName: 'Brave Search' };
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
      else if (key === 'r') action = event.shiftKey ? 'reload_fresh' : 'reload';
      else if (key === 'f5') action = 'reload_fresh';
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
  let resourceFailureReported = false;
  window.addEventListener('error', event => {
    if (resourceFailureReported || !event.isTrusted || window.top !== window
      || event.target?.tagName !== 'SCRIPT' || !event.target.src
      || !navigator.serviceWorker?.controller) return;
    const resource = new URL(event.target.src, location.href);
    if (!['http:', 'https:'].includes(resource.protocol) || resource.origin !== location.origin) return;
    resourceFailureReported = true;
    postShortcut?.(stringify({ __proto__: null, action: 'resource_load_failed', document_url: location.href,
      resource_url: resource.href, shortcut_token: '__SHORTCUT_TOKEN__' }));
  }, true);
})();
