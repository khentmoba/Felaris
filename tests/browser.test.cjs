const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const messages = [];
let keydown;
const window = { ipc: { postMessage: json => messages.push(JSON.parse(json)) }, addEventListener: (_, handler) => { keydown = handler; } };
const context = vm.createContext({ window });
vm.runInContext(fs.readFileSync('src/browser.js', 'utf8'), context);
const resolve = window.Felaris.resolveAddress;
for (const [input, expected] of [
  ['', null], ['  ', null], ['felaris://newtab', 'felaris://newtab'],
  ['example.com/docs', 'https://example.com/docs'], ['HTTPS://example.com', 'HTTPS://example.com'],
  ['localhost:5173', 'http://localhost:5173'], ['127.0.0.1:8080/a', 'http://127.0.0.1:8080/a'],
  ['[::1]:8080', 'http://[::1]:8080'], ['file:///C:/test.html', 'file:///C:/test.html'],
  ['a.b search', 'https://search.brave.com/search?q=a.b%20search'],
  ['rust & café #東京', 'https://search.brave.com/search?q=rust%20%26%20caf%C3%A9%20%23%E6%9D%B1%E4%BA%AC'],
  ['javascript:alert(1)', 'https://search.brave.com/search?q=javascript%3Aalert(1)'],
]) assert.equal(resolve(input), expected, input);
function press(key, modifiers = {}) {
  let prevented = false;
  keydown({ key, isTrusted: true, ...modifiers, preventDefault() { prevented = true; } });
  assert.ok(prevented);
  const { shortcut_token, ...message } = messages.pop();
  assert.equal(shortcut_token, '__SHORTCUT_TOKEN__');
  return message;
}
assert.deepEqual(press('T', { ctrlKey: true, shiftKey: true }), { action: 'restore_closed_tab' });
assert.deepEqual(press('S', { ctrlKey: true, shiftKey: true }), { action: 'sleep_inactive_tabs' });
assert.deepEqual(press('t', { ctrlKey: true }), { action: 'new_tab' });
assert.deepEqual(press('l', { ctrlKey: true }), { action: 'focus_address' });
assert.deepEqual(press('w', { ctrlKey: true }), { action: 'close_active_tab' });
assert.deepEqual(press('Tab', { ctrlKey: true, shiftKey: true }), { action: 'cycle_tab', backwards: true });
assert.deepEqual(press('2', { altKey: true }), { action: 'switch_workspace', id: 2 });
assert.deepEqual(press('F5'), { action: 'reload' });
vm.runInContext("Object.prototype.toJSON = function () { throw Error('page serialization hook'); }", context);
assert.deepEqual(press('t', { ctrlKey: true }), { action: 'new_tab' });
keydown({ key: 't', ctrlKey: true, isTrusted: true, isComposing: true });
keydown({ key: 't', ctrlKey: true, isTrusted: false });
assert.equal(messages.length, 0);
for (const file of ['src/ui.html', 'src/start_page.html', 'src/new_tab.html']) {
  const html = fs.readFileSync(file, 'utf8').replace('__COMMON_SCRIPT__', fs.readFileSync('src/browser.js', 'utf8'));
  for (const script of html.matchAll(/<script>([\s\S]*?)<\/script>/g)) new vm.Script(script[1], { filename: file });
  assert.ok(!html.includes('__COMMON_SCRIPT__'));
}
console.log('Address resolution, shortcut routing, and embedded script syntax passed.');
