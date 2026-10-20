import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { connect as connectView, closeConnections, writeDiagnostics } from './native_cdp.mjs';

const [chromePort, contentPort, origin, artifacts] = process.argv.slice(2);
const connect = (matches, port = chromePort) => connectView(matches, port);
async function until(probe, label) {
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    const value = await probe();
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

try {
  const sidebar = await connect(page => page.title === 'Felaris');
  await sidebar.call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
  const menuOpacity = await sidebar.evaluate(`(() => {
    const menu = document.getElementById('app-menu');
    menu.open = true;
    const panel = menu.querySelector('.menu');
    const animation = panel.getAnimations()[0];
    const opacity = [0, 0.25, 0.75, 1].map(progress => {
      if (animation) { animation.pause(); animation.currentTime = progress * animation.effect.getTiming().duration; }
      return getComputedStyle(panel).opacity;
    });
    animation?.finish();
    menu.open = false;
    return opacity;
  })()`);
  assert.deepEqual(menuOpacity, ['1', '1', '1', '1'], 'App menu must stay opaque throughout its opening animation');
  await sidebar.call('Emulation.setEmulatedMedia', { features: [] });
  const command = (action, payload = {}) => sidebar.evaluate(`window.ipc.postMessage(${JSON.stringify(JSON.stringify({ action, ...payload }))});true`);
  const palette = await connect(page => page.title.includes('New tab search'));
  const count = await sidebar.evaluate('currentState.tabs.length');
  await command('new_tab', { url: origin + '/keyboard' });
  const content = await connect(page => page.url === origin + '/keyboard', contentPort);
  await until(() => content.evaluate(`location.href === ${JSON.stringify(origin + '/keyboard')} && document.readyState === 'complete' && !!window.Felaris`), 'keyboard fixture');
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
  await until(() => submitted.evaluate(`location.href === ${JSON.stringify(origin + '/submitted')} && document.readyState === 'complete' && !!window.Felaris`), 'submitted page ready for keyboard input');
  await submitted.call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'w', modifiers: 2, windowsVirtualKeyCode: 87 }, { allowTargetClose: true });
  await until(() => sidebar.evaluate(`currentState.tabs.length === ${count + 1}`), 'Ctrl+W closes tab');
  await content.call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'w', modifiers: 2, windowsVirtualKeyCode: 87 });
  await press(content, 'T', 10);
  await until(() => sidebar.evaluate(`currentState.tabs.length === ${count + 2}`), 'Ctrl+Shift+T restores tab');
  const restored = await connect(page => page.url === origin + '/submitted', contentPort);
  await until(() => restored.evaluate(`location.href === ${JSON.stringify(origin + '/submitted')} && document.readyState === 'complete' && !!window.Felaris`), 'restored page ready for keyboard input');
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
  const beforePageIpc = await sidebar.evaluate('currentState.tabs.length');
  await restored.evaluate('Felaris.send("new_tab"); Felaris.send("duplicate_tab", { id: 1 }); Felaris.send("move_tab_to_workspace", { id: 1, workspace_id: 2 }); Felaris.send("expand_popup", { url: "https://example.com" })');
  await new Promise(resolve => setTimeout(resolve, 200));
  assert.equal(await sidebar.evaluate('currentState.tabs.length'), beforePageIpc, 'untrusted page IPC cannot create or mutate tabs');
  const popupBar = await connect(page => page.title === 'Felaris Popup');
  await restored.evaluate(`window.authDone = false; window.open(${JSON.stringify(origin + '/oauth/login')}, 'oauth', 'width=480,height=620'); true`);
  const oauthPopup = await connect(page => page.url === origin + '/oauth/login', contentPort);
  await until(() => oauthPopup.evaluate('document.readyState === "complete" && !!window.opener'), 'OAuth popup has live window.opener');
  await until(() => popupBar.evaluate('document.getElementById("address").textContent.includes("/oauth/login")'), 'Popup URL bar displays login URL');
  assert.equal(await sidebar.evaluate('currentState.tabs.length'), beforePageIpc, 'OAuth popup does not clutter tab list');
  await oauthPopup.evaluate('window.opener.authDone = true; window.close()');
  await until(() => restored.evaluate('window.authDone === true'), 'OAuth popup communicates with opener and closes');
  console.log('Native keyboard, palette open/cancel/submit, page retention, restoration, compact address focus, workspaces, pinning, reduced motion, OAuth popup, and IPC guard passed.');

  await command('new_tab');
  await until(() => palette.evaluate('!input.readOnly'), 'open palette for pointer regression');
  await palette.evaluate(`window.openNewTab({tabs:[],shortcuts:[{title:'Facebook',url:'${origin}/facebook'},{title:'X',url:'${origin}/x'}]},[])`);
  const xRow = await palette.evaluate('(()=>{const r=document.querySelectorAll(".option")[1].getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()');
  await palette.call('Input.dispatchMouseEvent', { type: 'mouseMoved', ...xRow });
  assert.equal(await palette.evaluate('document.querySelector(".option[aria-selected=true] .option-title").textContent'), 'X');
  await press(palette, 'Enter');
  await until(() => sidebar.evaluate(`currentState.tabs.find(tab=>tab.id===currentState.active_tab_id)?.url === '${origin}/x'`), 'Enter opens pointer-selected X result');
  console.log('Native pointer movement selects X and Enter opens the same result.');

  const permission = await connect(page => page.title === 'Felaris — Site permission');
  for (const [site, allow] of [[origin, false], [origin.replace('127.0.0.1', 'localhost'), true]]) {
    await command('new_tab', { url: site + '/permission' });
    const requester = await connect(page => page.url === site + '/permission', contentPort);
    await until(() => requester.evaluate('document.readyState === "complete"'), 'permission fixture ready');
    await requester.call('Runtime.evaluate', { expression: 'window.permissionResult="pending";Notification.requestPermission().then(result=>window.permissionResult=result);true', userGesture: true });
    await until(() => permission.evaluate(`current?.origin === ${JSON.stringify(site)} && current.kind === 'notifications'`), 'notification permission prompt');
    assert.equal(await permission.evaluate('document.getElementById("title").textContent'), 'Show notifications?');
    assert.equal(await permission.evaluate('document.getElementById("origin").textContent'), site);
    assert.equal(await permission.evaluate('document.activeElement.id'), 'block');
    if (artifacts && !allow) {
      const screenshot = await permission.call('Page.captureScreenshot', { format: 'png' });
      await writeFile(join(artifacts, 'permission.png'), Buffer.from(screenshot.data, 'base64'));
    }
    await permission.evaluate(`document.getElementById('${allow ? 'allow' : 'block'}').click()`);
    await until(() => requester.evaluate(`window.permissionResult === '${allow ? 'granted' : 'denied'}'`), 'native permission decision applied');
  }
  console.log('Native notification permission prompts show the requesting origin and apply Block and Allow.');

  await command('new_tab', { url: origin + '/location' });
  const locator = await connect(page => page.url === origin + '/location', contentPort);
  await until(() => locator.evaluate('document.readyState === "complete"'), 'location fixture ready');
  await locator.call('Runtime.evaluate', { expression: 'navigator.geolocation.getCurrentPosition(()=>{},()=>{});true', userGesture: true });
  await until(() => permission.evaluate('current?.kind === "location"'), 'location prompt');
  const staleToken = await permission.evaluate('current.token');
  await locator.evaluate(`window.ipc.postMessage(JSON.stringify({token:${staleToken},allow:true}));true`);
  assert.equal(await permission.evaluate('current.token'), staleToken, 'page IPC cannot answer a browser permission prompt');
  assert.notEqual(await locator.evaluate('navigator.permissions.query({name:"geolocation"}).then(result=>result.state)'), 'granted');
  await command('navigate', { url: origin + '/navigated' });
  await until(() => permission.evaluate('current === null'), 'navigation dismisses pending permission');
  await permission.evaluate(`window.ipc.postMessage(JSON.stringify({token:${staleToken},allow:true}));true`);
  assert.equal(await permission.evaluate('current'), null);
  await until(() => locator.evaluate(`location.href === '${origin}/navigated' && document.readyState === 'complete'`), 'navigation completes');
  assert.notEqual(await locator.evaluate('navigator.permissions.query({name:"geolocation"}).then(result=>result.state)'), 'granted', 'stale responses cannot grant a permission after navigation');
  console.log('Navigation dismisses a pending permission; stale responses cannot grant it.');

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
    assert.equal(result, '63 checks passed', `${width}x${height}, ${motion}`);
    assert.equal(await fixture.evaluate('window.checkResults.length'), 63);
    console.log(`${result}: ${width}x${height}, ${motion}`);
  }
} catch (error) {
  if (artifacts) {
    try { await writeDiagnostics([chromePort, contentPort], artifacts, error); }
    catch (failure) { console.error('Could not save native diagnostics:', failure); }
  }
  throw error;
} finally {
  closeConnections();
}
