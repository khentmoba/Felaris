import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const sockets = [];
const inputs = [];

export async function connect(matches, port) {
  const deadline = Date.now() + 15000;
  let target;
  while (Date.now() < deadline) {
    try {
      const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(2000) })).json();
      target = pages.find(matches);
      if (target) break;
    } catch (error) {
      if (error.cause?.code !== 'ECONNREFUSED') throw error;
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  if (!target) throw Error(`WebView target not ready on port ${port}`);
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
  socket.onclose = () => {
    for (const respond of pending.values()) respond({ targetClosed: true });
    pending.clear();
  };
  const call = (method, params = {}, { allowTargetClose = false } = {}) => new Promise((resolve, reject) => {
    if (method.startsWith('Input.')) {
      inputs.push({ time: Date.now(), target: target.title, method, params });
      if (inputs.length > 20) inputs.shift();
    }
    const request = ++id;
    const timer = setTimeout(() => { pending.delete(request); reject(Error('CDP timed out: ' + method)); }, 10000);
    pending.set(request, response => {
      clearTimeout(timer);
      if (response.targetClosed && !allowTargetClose) reject(Error(`WebView closed during ${method}`));
      else if (response.error) reject(Error(JSON.stringify(response.error)));
      else resolve(response.result);
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

export async function writeDiagnostics(ports, directory, error, screenshots = false) {
  const targets = [];
  for (const port of ports) {
    try {
      const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(2000) })).json();
      for (const target of pages.filter(page => page.type === 'page')) {
        const entry = { port, id: target.id, title: target.title, url: target.url.startsWith('data:') ? 'data:' : target.url };
        targets.push(entry);
        try {
          const view = await connect(page => page.id === target.id, port);
          entry.state = await view.evaluate(`(() => {
            const state = typeof currentState === 'object' ? currentState : null;
            const tab = state?.tabs?.find(tab => tab.id === state.active_tab_id);
            return { ready: document.readyState, focused: document.hasFocus(), activeElement: document.activeElement?.id || document.activeElement?.tagName,
              width: innerWidth, height: innerHeight, reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
              sidebarCollapsed: state?.sidebar_collapsed, sidebarPeek: state?.sidebar_peek,
              activeTab: tab && { id: tab.id, title: tab.title, url: tab.url, loading: tab.is_loading, sleeping: tab.is_sleeping },
              shortcutBridge: !!window.Veluno, inputReadOnly: document.querySelector('input')?.readOnly };
          })()`);
          if (screenshots && (target.title === 'Veluno' || target.url.startsWith('http:'))) {
            const shot = await view.call('Page.captureScreenshot', { format: 'png' });
            entry.screenshot = `view-${targets.length}.png`;
            await writeFile(join(directory, entry.screenshot), Buffer.from(shot.data, 'base64'));
          }
        } catch (failure) { entry.error = String(failure); }
      }
    } catch (failure) { targets.push({ port, error: String(failure) }); }
  }
  await writeFile(join(directory, 'debugger.json'), JSON.stringify({ error: error?.stack, inputs, targets }, null, 2));
  return targets;
}

export function closeConnections() {
  for (const socket of sockets) socket.close();
}
