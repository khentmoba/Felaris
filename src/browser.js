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
    const words=needle.split(/\s+/);
    function relevance(option) {
     if(!needle) return 0;
     const title=(option.title||'').toLowerCase();
     const url=option.url.toLowerCase();
     if(!words.every(word=>(title+' '+url).includes(word))) return -1;
     let host='';
     try { host=new URL(option.url).hostname.replace(/^www\./,'').toLowerCase(); } catch {}
     if(needle===host||needle===host.split('.')[0]) return 100;
     if(needle===title) return 95;
     if(needle.length>=2&&host.startsWith(needle)) return 90;
     if(needle.length>=2&&title.startsWith(needle)) return 80;
     return 40;
    }
    function add(option) {
     if(!option.url||seen.has(norm(option.url))) return;
     const score=relevance(option);
     if(score<0) return;
     seen.add(norm(option.url));
     options.push({option,score});
    }
    let fallback;
    if(target) {
     const search=target===Felaris.searchUrl+encodeURIComponent(query);
     fallback={option:{title:query,detail:search?'Search with '+Felaris.searchName.replace(/ Search$/,''):'Open in a new tab',url:target,search},score:search?60:110};
     seen.add(norm(target));
    }
    for(const tab of (state.tabs||[]).filter(tab=>!['felaris://newtab','felaris://blank'].includes(tab.url)).slice(0,needle?Infinity:6)) {
     const workspace=(state.workspaces||[]).find(space=>space.id===tab.workspace_id);
     add({title:tab.title||tab.url,detail:`Switch to tab${workspace?' · '+workspace.name:''} · ${tab.url}`,id:tab.id,url:tab.url,favicon:tab.favicon});
    }
    for(const entry of (state.bookmarks||[])) {
     if(!entry?.url) continue;
     add({title:entry.title||entry.url,detail:`Bookmark · ${entry.url}`,url:entry.url,bookmark:true});
    }
    for(const entry of historyEntries) {
     if(!entry?.url) continue;
     add({title:entry.title||entry.url,detail:`History · ${entry.url}`,url:entry.url,history:true});
    }
    for(const item of (state.shortcuts||[])) {
     if(!item?.url) continue;
     add({title:item.title||item.url,detail:`Shortcut · ${item.url}`,url:item.url,favicon:item.favicon,shortcut:true});
    }
    options.sort((a,b)=>b.score-a.score);
    const ranked=options.slice(0,fallback?7:8);
    if(fallback) ranked.push(fallback);
    return ranked.sort((a,b)=>b.score-a.score).map(item=>item.option);
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
      else if (key === 'delete' && event.shiftKey) { action = 'open_settings'; payload = { section: 'privacy' }; }
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
      else if (key === 'f9') action = 'toggle_reader';
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
