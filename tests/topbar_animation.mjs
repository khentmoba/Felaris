import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { connect, closeConnections } from './native_cdp.mjs';

const [port, artifacts] = process.argv.slice(2);
try {
  const sidebar = await connect(page => page.title === 'Felaris', port);
  const deadline = Date.now() + 15000;
  let topbar;
  while (!topbar && Date.now() < deadline) {
    const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    for (const page of pages.filter(page => page.type === 'page' && page.title !== 'Felaris')) {
      const candidate = await connect(target => target.id === page.id, port);
      if (await candidate.evaluate("!!document.getElementById('drag') && !!window.setLoading")) { topbar = candidate; break; }
    }
    if (!topbar) await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert.ok(topbar, 'Top bar target missing');
  await sidebar.evaluate(`(async()=>{
    while(!currentState.tabs.length) await new Promise(resolve=>setTimeout(resolve,20));
    if(!currentState.sidebar_collapsed) window.ipc.postMessage(JSON.stringify({action:'toggle_sidebar'}));
    return true;
  })()`);
  await topbar.evaluate(`document.documentElement.dispatchEvent(new MouseEvent('mouseenter')); true`);
  await new Promise(resolve => setTimeout(resolve, 500));
  await topbar.evaluate(`window.widths=[]; window.sampling=setInterval(()=>widths.push(innerWidth),10); true`);
  await sidebar.evaluate(`(async()=>{
    const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
    for(let i=0;i<3;i++) {
      document.documentElement.dispatchEvent(new MouseEvent('mouseenter'));
      await wait(400);
      document.documentElement.dispatchEvent(new MouseEvent('mouseleave'));
      await wait(600);
    }
    document.documentElement.dispatchEvent(new MouseEvent('mouseenter'));
    await wait(80);
    document.documentElement.dispatchEvent(new MouseEvent('mouseleave'));
    await wait(300);
    document.documentElement.dispatchEvent(new MouseEvent('mouseenter'));
    await wait(400);
    return true;
  })()`);
  const result = await topbar.evaluate(`clearInterval(sampling); ({widths,open:document.body.classList.contains('open'),height:innerHeight})`);
  await writeFile(join(artifacts, 'topbar-animation.json'), JSON.stringify(result, null, 2));
  assert.ok(result.open && result.height >= 34, 'Top bar must remain open throughout sidebar animation');
  assert.ok(result.widths.length > 100, 'Must sample multiple reveal/hide/reversal cycles');
  assert.equal(new Set(result.widths).size, 1, `Sidebar animation resized top bar: ${Math.min(...result.widths)}–${Math.max(...result.widths)}px`);
  console.log(`Top bar width stayed at ${result.widths[0]}px over ${result.widths.length} samples during reveal, hide, and reversal`);
} finally {
  closeConnections();
}
