const fs = require('node:fs');
const os = require('node:os');
const net = require('node:net');
const http = require('node:http');
const path = require('node:path');
const { spawn } = require('node:child_process');
const assert = require('node:assert/strict');
fs.mkdirSync('.impeccable/review', { recursive: true });
const watchdog = setTimeout(() => { console.error('Native smoke test timed out'); process.exit(1); }, 40000);
watchdog.unref();
function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(err => err ? reject(err) : resolve(port));
    });
    server.on('error', reject);
  });
}
function findBinary() {
  const candidates = [
    process.env.FELARIS_EXE,
    process.env.CARGO_TARGET_DIR && path.join(process.env.CARGO_TARGET_DIR, 'debug', 'felaris.exe'),
    path.join('target', 'debug', 'felaris.exe'),
    path.join('target', 'release', 'felaris.exe'),
  ].filter(Boolean);
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }
  throw Error('Could not find compiled felaris.exe (build with cargo build first or set FELARIS_EXE)');
}
async function connect(target) {
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  let loads = 0, next = 0;
  const pending = new Map();
  ws.onmessage = event => {
    const message = JSON.parse(event.data);
    if (message.method === 'Page.loadEventFired') loads++;
    const request = pending.get(message.id);
    if (request) {
      pending.delete(message.id);
      message.error ? request.reject(message.error) : request.resolve(message.result);
    }
  };
  const call = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++next;
    const timer = setTimeout(() => { pending.delete(id); reject(Error('CDP timed out: ' + method)); }, 5000);
    pending.set(id, {
      resolve: value => { clearTimeout(timer); resolve(value); },
      reject: error => { clearTimeout(timer); reject(error); },
    });
    ws.send(JSON.stringify({ id, method, params }));
  });
  return {
    call, loads: () => loads, close: () => ws.close(),
    evaluate: async expression => {
      const result = await call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
      if (result.exceptionDetails) throw Error(JSON.stringify(result.exceptionDetails));
      return result.result.value;
    },
  };
}
async function until(fn, label) {
  for (let i = 0; i < 50; i++) {
    const value = await fn();
    if (value) return value;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw Error('Timed out: ' + label);
}
async function press(view, key, modifiers = 0) {
  const params = { key, modifiers, windowsVirtualKeyCode: key === 'Escape' ? 27 : key === 'Enter' ? 13 : key.toUpperCase().charCodeAt(0) };
  if (key === 'Enter') params.text = '\r';
  await view.call('Input.dispatchKeyEvent', { type: 'keyDown', ...params });
  await view.call('Input.dispatchKeyEvent', { type: 'keyUp', ...params });
}
(async () => {
  let port = process.env.FELARIS_CDP_PORT ? Number(process.env.FELARIS_CDP_PORT) : null;
  let child = null;
  let tempExe = null;
  if (!port) {
    try {
      const existing = await fetch('http://127.0.0.1:9223/json/list').then(r => r.json());
      if (Array.isArray(existing) && existing.length > 0) port = 9223;
    } catch {}
  }
  if (!port) {
    port = await getFreePort();
    const srcExe = findBinary();
    tempExe = path.join(os.tmpdir(), `felaris-smoke-${process.pid}-${Date.now()}.exe`);
    fs.copyFileSync(srcExe, tempExe);
    child = spawn(tempExe, [], {
      env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}` },
      stdio: 'ignore',
    });
  }
  let sidebar, content, palette, httpServer;
  try {
    const targets = await until(async () => {
      try {
        const pages = await fetch(`http://127.0.0.1:${port}/json/list`).then(response => response.json());
        return pages.some(page => page.title.includes('New tab search')) && pages.some(page => page.title === 'Felaris') ? pages : null;
      } catch { return null; }
    }, 'native startup');
    sidebar = await connect(targets.find(page => page.title === 'Felaris'));
    content = await connect(targets.find(page => page.title === 'New Tab'));
    palette = await connect(targets.find(page => page.title.includes('New tab search')));
    await content.call('Page.enable');
    const key = async (key, modifiers, navigates = true) => {
      const before = content.loads();
      await press(content, key, modifiers);
      if (navigates) await until(() => content.loads() > before, 'content load');
    };
    const paletteFocused = () => palette.evaluate('document.hasFocus() && document.activeElement.id === "new-tab-input"');
    assert.equal(await sidebar.evaluate('currentState.tabs.length'), 1);
    assert.equal(await sidebar.evaluate('document.querySelector(".tab-main use")?.getAttribute("href")'), '#felaris');
    assert.equal(await sidebar.evaluate('!!document.querySelector(".essentials,.essential")'), false);
    assert.equal(await sidebar.evaluate('getComputedStyle(document.body).backgroundColor'), 'rgb(16, 16, 17)');
    for (const view of [sidebar, content, palette]) {
      assert.equal(await view.evaluate('Felaris.resolveAddress("rust & café")'), 'https://search.brave.com/search?q=rust%20%26%20caf%C3%A9');
    }
    await content.evaluate('window.pageMarker = "kept"');
    await key('t', 2, false);
    await until(paletteFocused, 'Ctrl+T opens palette');
    assert.equal(await sidebar.evaluate('currentState.tabs.length'), 1);
    await palette.evaluate('document.getElementById("new-tab-input").value = "draft query"');
    await press(palette, 'Escape');
    await until(() => content.evaluate('document.hasFocus() && window.pageMarker === "kept"'), 'Escape preserves page and restores focus');
    assert.equal(await sidebar.evaluate('currentState.tabs.length'), 1);

    await sidebar.evaluate('document.getElementById("btn-add-tab").click()');
    await until(paletteFocused, 'sidebar New tab opens palette');
    assert.equal(await palette.evaluate('document.getElementById("new-tab-input").value'), '');
    await palette.evaluate('input.value="felaris://newtab"; input.dispatchEvent(new Event("input"))');
    const beforeSubmit = content.loads();
    await press(palette, 'Enter');
    await until(() => content.loads() > beforeSubmit, 'Enter loads destination');
    await until(() => sidebar.evaluate('currentState.tabs.length === 2'), 'Enter creates one tab');
    assert.equal(await palette.evaluate('document.hasFocus()'), false);

    await key('w', 2);
    await until(() => sidebar.evaluate('currentState.tabs.length === 1 && currentState.can_restore_closed_tab'), 'close active tab');
    await key('T', 10);
    await until(() => sidebar.evaluate('currentState.tabs.length === 2'), 'restore shortcut does not open palette');
    await sidebar.evaluate('document.getElementById("btn-pin-tab").click()');
    await until(() => sidebar.evaluate('document.querySelectorAll("#pinned-tabs .tab").length === 1'), 'pinning');
    await key('2', 1);
    await until(() => sidebar.evaluate('currentState.active_workspace_id === 2'), 'workspace');
    await key('s', 2, false);
    await until(() => sidebar.evaluate('currentState.sidebar_collapsed'), 'collapse');
    await key('t', 2, false);
    await until(paletteFocused, 'palette works with compact rail');
    assert.equal(await sidebar.evaluate('currentState.sidebar_collapsed'), true);
    await palette.evaluate('Promise.all(document.getAnimations().map(animation => animation.finished.catch(() => {}))).then(() => true)');
    const screenshot = await palette.call('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync('.impeccable/review/native-palette.png', Buffer.from(screenshot.data, 'base64'));
    await press(palette, 'Escape');
    await until(() => content.evaluate('document.hasFocus()'), 'compact cancel');
    await key('l', 2, false);
    await until(() => sidebar.evaluate('!currentState.sidebar_collapsed && document.activeElement.id === "url-input"'), 'address focus expands sidebar');
    await key('1', 1);
    await until(() => sidebar.evaluate('currentState.active_workspace_id === 1'), 'return workspace');
    await content.evaluate('Felaris.send("new_tab")');
    await new Promise(resolve => setTimeout(resolve, 150));
    assert.equal(await sidebar.evaluate('currentState.tabs.length'), 3);
    assert.equal(await palette.evaluate('document.hasFocus()'), false);
    await sidebar.call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
    assert.equal(await sidebar.evaluate('(() => {const snapshot=structuredClone(currentState);snapshot.tabs.push({...snapshot.tabs[0],id:999});snapshot.is_loading=true;updateState(snapshot);return document.getAnimations().filter(animation=>animation.id==="tab-layout"&&animation.playState==="running").length;})()'), 0);
    assert.equal(await sidebar.evaluate('getComputedStyle(document.querySelector(".progress")).animationName'), 'none');
    await palette.call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
    await key('t', 2, false);
    await until(paletteFocused, 'reduced-motion palette');
    assert.equal(await palette.evaluate('document.getAnimations().filter(animation => animation.id.startsWith("palette-") && animation.playState === "running").length'), 0);
    await press(palette, 'Escape');
    await until(() => content.evaluate('document.hasFocus()'), 'reduced-motion cancel');

    httpServer = http.createServer((req, res) => {
      const cleanPath = path.normalize(decodeURIComponent((req.url || '/').split('?')[0])).replace(/^(\.\.(\/|\\|$))+/, '');
      const filePath = path.join(process.cwd(), cleanPath);
      if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
        res.writeHead(404).end();
        return;
      }
      const ext = path.extname(filePath);
      const mime = ext === '.html' ? 'text/html; charset=utf-8' : ext === '.js' ? 'text/javascript; charset=utf-8' : 'application/octet-stream';
      res.writeHead(200, { 'Content-Type': mime }).end(fs.readFileSync(filePath));
    });
    await new Promise((resolve, reject) => {
      httpServer.listen(0, '127.0.0.1', resolve);
      httpServer.on('error', reject);
    });
    const httpPort = httpServer.address().port;
    await content.call('Page.navigate', { url: `http://127.0.0.1:${httpPort}/tests/ui.html` });
    const uiResult = await until(async () => {
      const text = await content.evaluate('document.getElementById("results")?.textContent || ""').catch(() => '');
      return text && text !== 'Running…' ? text : null;
    }, 'tests/ui.html completion');
    assert.equal(uiResult, '44 checks passed');

    console.log('PASS: Shortcut dock removed, black theme, #felaris icon, animated palette lifecycle, reduced motion, cancel without reload, submit, compact mode, shortcuts, restoration, workspaces, IPC guard, and 44 UI DOM checks.');
  } catch (error) {
    if (sidebar && content && palette) {
      console.error('Native failure state:', await Promise.allSettled([
        sidebar.evaluate('({tabs:currentState.tabs.length,active:currentState.active_tab_id})'),
        content.evaluate('({focused:document.hasFocus(),pagePreserved:window.pageMarker==="kept",visibility:document.visibilityState})'),
        palette.evaluate('({focused:document.hasFocus(),submitting,visibility:document.visibilityState,animations:document.getAnimations().map(a=>({id:a.id,state:a.playState,time:a.currentTime}))})'),
      ]));
    }
    throw error;
  } finally {
    httpServer?.close();
    sidebar?.close();
    content?.close();
    palette?.close();
    if (child) {
      child.kill();
      await new Promise(resolve => setTimeout(resolve, 250));
    }
    if (tempExe) {
      try { fs.unlinkSync(tempExe); } catch {}
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
