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
context.Felaris=window.Felaris;
const suggest=window.Felaris.suggestions;
const sources={tabs:[{id:1,title:'YouTube tab',url:'https://youtube.com'}],bookmarks:[{title:'Duplicate',url:'https://youtube.com/'}],shortcuts:[{title:'YouTube',url:'https://youtube.com'},{title:'GitHub',url:'https://github.com'}]};
const suggestions=suggest('you',sources,[{title:'YouTube Studio',url:'https://studio.youtube.com'}]);
assert.equal(suggestions[0].url,'https://search.brave.com/search?q=you');
assert.equal(suggestions[1].id,1);
assert.equal(suggestions[2].title,'YouTube Studio');
assert.equal(suggestions.length,3,'Deduplicate suggestions');
assert.equal(suggest('',{shortcuts:sources.shortcuts})[0].title,'YouTube');
assert.equal(suggest('no match',sources).length,1);
assert.equal(suggest('github.com',{})[0].url,'https://github.com');
assert.equal(suggest('',{shortcuts:Array.from({length:12},(_,i)=>({title:String(i),url:'https://example.com/'+i}))}).length,8);
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
assert.deepEqual(press('B', { ctrlKey: true, shiftKey: true }), { action: 'toggle_sidebar' });
assert.deepEqual(press('N', { ctrlKey: true, shiftKey: true }), { action: 'new_private_tab' });
assert.deepEqual(press('d', { ctrlKey: true }), { action: 'toggle_bookmark' });
assert.deepEqual(press('l', { ctrlKey: true }), { action: 'focus_address' });
assert.deepEqual(press('j', { ctrlKey: true }), { action: 'open_downloads' });
assert.deepEqual(press('w', { ctrlKey: true }), { action: 'close_active_tab' });
assert.deepEqual(press('Tab', { ctrlKey: true, shiftKey: true }), { action: 'cycle_tab', backwards: true });
assert.deepEqual(press('=', { ctrlKey: true }), { action: 'zoom', delta: 10 });
assert.deepEqual(press('-', { ctrlKey: true }), { action: 'zoom', delta: -10 });
assert.deepEqual(press('0', { ctrlKey: true }), { action: 'zoom', delta: 0 });
assert.deepEqual(press('2', { altKey: true }), { action: 'switch_profile', index: 2 });
assert.deepEqual(press('F5'), { action: 'reload' });
assert.deepEqual(press('F11'), { action: 'toggle_fullscreen' });
vm.runInContext("Object.prototype.toJSON = function () { throw Error('page serialization hook'); }", context);
assert.deepEqual(press('t', { ctrlKey: true }), { action: 'new_tab' });
keydown({ key: 't', ctrlKey: true, isTrusted: true, isComposing: true });
keydown({ key: 't', ctrlKey: true, isTrusted: false });
keydown({ key: 's', ctrlKey: true, isTrusted: true, preventDefault() { throw Error('Ctrl+S belongs to the page'); } });
assert.equal(messages.length, 0);
for (const file of ['src/ui.html', 'src/start_page.html', 'src/new_tab.html', 'src/topbar.html', 'src/popup.html', 'src/permission.html', 'src/history.html', 'src/settings.html']) {
  const html = fs.readFileSync(file, 'utf8').replace('__COMMON_SCRIPT__', fs.readFileSync('src/browser.js', 'utf8')).replace('__SHORTCUTS__', '[]');
  for (const script of html.matchAll(/<script>([\s\S]*?)<\/script>/g)) new vm.Script(script[1], { filename: file });
  assert.ok(!html.includes('__COMMON_SCRIPT__'));
}
const uiHtml = fs.readFileSync('src/ui.html', 'utf8');
assert.ok(uiHtml.includes('<symbol id="felaris"'), 'src/ui.html must define #felaris logo symbol');
assert.ok(uiHtml.includes("icon('felaris')"), 'src/ui.html must render #felaris icon for felaris://newtab');
const expectedAssets = [
  'app-icon-polaris.jpg',
  'favicon.svg',
  'felaris-icon.png',
  'icon.ico',
  'icon_64.rgba',
  'logo-monochrome.svg',
  'logo-wordmark.svg',
  'logo.svg',
];
assert.deepEqual(fs.readdirSync('assets').sort(), expectedAssets, 'assets/ must contain only canonical branding files');
for (const required of ['scripts/build.py', 'scripts/generate-icons.py', 'scripts/notify_discord.py', '.agents/skills/discord-release/SKILL.md']) {
  assert.ok(fs.existsSync(required), `Missing tracked release file: ${required}`);
}
assert.ok(
  fs.readFileSync('scripts/notify_discord.py', 'utf8').includes('LOCALAPPDATA'),
  'scripts/notify_discord.py must support LOCALAPPDATA/Felaris/discord_webhook.txt fallback across worktrees',
);
console.log('Address resolution, shortcut routing, asset hygiene, and embedded script syntax passed.');
require('./sleep.test.cjs');
