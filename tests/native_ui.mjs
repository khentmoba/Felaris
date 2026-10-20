import assert from 'node:assert/strict';

const [chromePort, contentPort, origin] = process.argv.slice(2);
const sockets = [];
async function until(probe, label) {
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    const value = await probe();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  let info='';try{info=await (globalThis.__sidebar?.evaluate("JSON.stringify({c:currentState.sidebar_collapsed,peek:currentState.sidebar_peek,ae:document.activeElement.id||document.activeElement.tagName,cls:document.body.className,vis:getComputedStyle(document.getElementById('url-input')).visibility,w:innerWidth,h:innerHeight})"))}catch(e){info=String(e)}
  throw Error('Timed out: ' + label + ' ' + info);
}
async function connect(matches, port = chromePort) {
  const target = await until(async () => {
    const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    return pages.find(matches);
  }, 'WebView target');
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  sockets.push(socket);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let id = 0;
  const pending = new Map();
  socket.onmessage = event => {
    const response = JSON.parse(event.data);
    pending.get(response.id)?.(response);
    pending.delete(response.id);
  };
  const call = (method, params = {}) => new Promise((resolve, reject) => {
    const request = ++id;
    const timer = setTimeout(() => { pending.delete(request); reject(Error('CDP timed out: ' + method)); }, 10000);
    pending.set(request, response => {
      clearTimeout(timer);
      response.error ? reject(Error(JSON.stringify(response.error))) : resolve(response.result);
    });
    socket.send(JSON.stringify({ id: request, method, params }));
  });
  return {
    call,
    evaluate: async expression => {
      const result = await call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
      if (result.exceptionDetails) throw Error(JSON.stringify(result.exceptionDetails));
      return result.result.value;
    },
  };
}
async function press(view, key, modifiers = 0) {
  const params = { key, modifiers, windowsVirtualKeyCode: key === 'Escape' ? 27 : key === 'Enter' ? 13 : key.toUpperCase().charCodeAt(0) };
  if (key === 'Enter') params.text = '\r';
  await view.call('Input.dispatchKeyEvent', { type: 'keyDown', ...params });
  await view.call('Input.dispatchKeyEvent', { type: 'keyUp', ...params });
}

try {
  const sidebar = await connect(page => page.title === 'Felaris'); globalThis.__sidebar = sidebar;
  const command = (action, payload = {}) => sidebar.evaluate(`window.ipc.postMessage(${JSON.stringify(JSON.stringify({ action, ...payload }))});true`);
  const palette = await connect(page => page.title.includes('New tab search'));
  const count = await sidebar.evaluate('currentState.tabs.length');
  await command('new_tab', { url: origin + '/keyboard' });
  const content = await connect(page => page.url === origin + '/keyboard', contentPort);
  await until(() => content.evaluate('document.readyState === "complete"'), 'keyboard fixture');
  await content.evaluate('window.pageMarker = "kept"');
  await press(content, 't', 2);
  await until(() => palette.evaluate('document.activeElement.id === "new-tab-input" && !input.readOnly'), 'Ctrl+T opens palette');
  assert.equal(await sidebar.evaluate('currentState.tabs.length'), count + 1);
  await palette.evaluate('input.value = "draft query"');
  await press(palette, 'Escape');
  await until(() => palette.evaluate('input.readOnly'), 'Escape dismisses palette');
  assert.equal(await content.evaluate('window.pageMarker'), 'kept');
  await sidebar.evaluate('document.getElementById("btn-add-tab").click()');
  await until(() => palette.evaluate('!input.readOnly && input.value === ""'), 'sidebar opens fresh palette');
  await palette.evaluate(`input.value=${JSON.stringify(origin + '/submitted')};input.dispatchEvent(new Event('input'))`);
  await press(palette, 'Enter');
  await until(() => sidebar.evaluate(`currentState.tabs.length === ${count + 2}`), 'Enter creates one tab');
  const submitted = await connect(page => page.url === origin + '/submitted', contentPort);
  await until(() => submitted.evaluate('document.readyState === "complete" && !!window.Felaris'), 'submitted page ready for keyboard input');
  await press(submitted, 'w', 2);
  await until(() => sidebar.evaluate(`currentState.tabs.length === ${count + 1}`), 'Ctrl+W closes tab');
  await press(content, 'T', 10);
  await until(() => sidebar.evaluate(`currentState.tabs.length === ${count + 2}`), 'Ctrl+Shift+T restores tab');
  const restored = await connect(page => page.url === origin + '/submitted', contentPort);
  await until(() => restored.evaluate('document.readyState === "complete" && !!window.Felaris'), 'restored page ready for keyboard input');
  await press(restored, 's', 2);
  await until(() => sidebar.evaluate('currentState.sidebar_collapsed'), 'Ctrl+S collapses sidebar');
  await press(restored, 'l', 2);
  await until(() => sidebar.evaluate('!currentState.sidebar_collapsed && document.activeElement.id === "url-input"'), 'Ctrl+L expands and selects address');
  await press(restored, '2', 1);
  await until(() => sidebar.evaluate('currentState.active_workspace_id === 2'), 'Alt+2 switches workspace');
  await press(sidebar, '1', 1);
  await until(() => sidebar.evaluate('currentState.active_workspace_id === 1'), 'Alt+1 returns to workspace');
  const pins = await sidebar.evaluate('document.querySelectorAll("#pinned-tabs .tab").length');
  const pinChange = await sidebar.evaluate('currentState.tabs.find(tab => tab.id === currentState.active_tab_id).is_pinned ? -1 : 1');
  await sidebar.evaluate('document.getElementById("btn-pin-tab").click()');
  await until(() => sidebar.evaluate(`document.querySelectorAll('#pinned-tabs .tab').length === ${pins + pinChange}`), 'toggle active tab pin');
  await sidebar.evaluate('document.getElementById("btn-pin-tab").click()');
  await palette.call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
  await press(sidebar, 't', 2);
  await until(() => palette.evaluate('!input.readOnly'), 'reduced-motion palette');
  assert.equal(await palette.evaluate('document.getAnimations().filter(animation => animation.id.startsWith("palette-") && animation.playState === "running").length'), 0);
  await press(palette, 'Escape');
  await until(() => palette.evaluate('input.readOnly'), 'reduced-motion cancel');
  await restored.evaluate('Felaris.send("new_tab")');
  await new Promise(resolve => setTimeout(resolve, 200));
  assert.equal(await sidebar.evaluate('currentState.tabs.length'), count + 2, 'untrusted page IPC cannot create tabs');
  console.log('Native keyboard, palette open/cancel/submit, page retention, restoration, compact address focus, workspaces, pinning, reduced motion, and IPC guard passed.');

  await command('new_tab', { url: origin + '/tests/ui.html' });
  const fixture = await connect(page => page.url === origin + '/tests/ui.html', contentPort);
  await fixture.call('Emulation.setFocusEmulationEnabled', { enabled: true });
  for (const [width, height, motion] of [[1380, 860, 'no-preference'], [640, 480, 'reduce']]) {
    await fixture.call('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
    await fixture.call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: motion }] });
    await fixture.call('Page.navigate', { url: `${origin}/tests/ui.html?size=${width}` });
    const result = await until(async () => {
      try {
        return await fixture.evaluate(`location.search === '?size=${width}' && (() => { const text=document.getElementById('results')?.textContent;return text?.includes('passed') || text?.startsWith('FAIL:') ? text : null })()`);
      } catch (error) {
        if (!error.message.includes('Execution context was destroyed')) throw error;
        return null;
      }
    }, 'DOM fixture completion');
    assert.equal(result, '48 checks passed', `${width}x${height}, ${motion}`);
    assert.equal(await fixture.evaluate('window.checkResults.length'), 48);
    console.log(`${result}: ${width}x${height}, ${motion}`);
  }
} finally {
  for (const socket of sockets) socket.close();
}
