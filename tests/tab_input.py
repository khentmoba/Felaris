"""Sleeping and waking tabs must release native hosts and retain clickable content."""
import argparse
import ctypes
from ctypes import wintypes
from http.server import ThreadingHTTPServer
import json
import subprocess
import threading
import time

from native_app import NativeApp
from native_smoke import Fixture


def check_tab_input(executable):
    app = NativeApp(executable, 'tab-input')
    server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    origin = f'http://127.0.0.1:{server.server_port}'
    threading.Thread(target=server.serve_forever, daemon=True).start()
    user32 = ctypes.WinDLL('user32', use_last_error=True)
    callback = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    user32.EnumWindows.argtypes = [callback, wintypes.LPARAM]
    user32.EnumChildWindows.argtypes = [wintypes.HWND, callback, wintypes.LPARAM]
    user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.GetParent.argtypes = [wintypes.HWND]
    user32.GetParent.restype = wintypes.HWND
    user32.GetClientRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
    user32.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]

    def sidebar(expression):
        result = subprocess.run(['node', 'tests/native_command.mjs', str(app.port), expression],
                                check=True, capture_output=True, text=True, timeout=20)
        return json.loads(result.stdout)

    def command(action, **payload):
        message = json.dumps({'action': action, **payload})
        sidebar(f'window.ipc.postMessage({json.dumps(message)});true')

    def wait(expression):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if sidebar(expression):
                return
            time.sleep(.1)
        raise AssertionError(f'Tab input regression timed out: {expression}')

    def hosts(root):
        result = []

        @callback
        def child(hwnd, _):
            name = ctypes.create_unicode_buffer(64)
            user32.GetClassNameW(hwnd, name, len(name))
            if user32.GetParent(hwnd) == root and name.value == 'WRY_WEBVIEW':
                result.append(hwnd)
            return True

        user32.EnumChildWindows(root, child, 0)
        return result

    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([origin + '/'], log)
            wait('!currentState.is_loading && currentState.tabs.some(tab=>tab.title==="Felaris fixture")')
            first = sidebar('currentState.active_tab_id')
            roots = []

            @callback
            def root(hwnd, _):
                pid = wintypes.DWORD()
                user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
                if pid.value == app.process.pid:
                    rect = wintypes.RECT()
                    user32.GetClientRect(hwnd, ctypes.byref(rect))
                    roots.append((rect.right, hwnd))
                return True

            user32.EnumWindows(root, 0)
            window = max(roots)[1]
            baseline = len(hosts(window))
            command('new_tab', url=origin + '/second')
            wait(f'!currentState.is_loading && currentState.active_tab_id!=={first} && currentState.tabs.find(tab=>tab.id===currentState.active_tab_id)?.title==="Felaris second"')
            second = sidebar('currentState.active_tab_id')
            assert len(hosts(window)) == baseline + 1, f'Host baseline {baseline}, after new tab {len(hosts(window))}'
            for selected, sleeping in [(second, first), (first, second)] * 3:
                command('switch_tab', id=selected)
                wait(f'currentState.active_tab_id==={selected} && !currentState.is_loading')
                command('sleep_inactive_tabs')
                wait(f'currentState.tabs.find(tab=>tab.id==={sleeping}).is_sleeping')
                actual = len(hosts(window))
                assert actual == baseline, f'Sleeping tab leaked a native window: expected {baseline}, got {actual}'
                url = origin + ('/' if selected == first else '/second')
                click = f"""import {{ connect, closeConnections }} from './tests/native_cdp.mjs';
try {{
 const page=await connect(target=>target.url==={json.dumps(url)}, {app.content_port});
 await page.evaluate(`window.testClicks=0;const button=document.createElement('button');button.id='click-check';button.textContent='Click';button.onclick=()=>window.testClicks++;document.body.prepend(button);`);
 const point=await page.evaluate(`(()=>{{const r=document.getElementById('click-check').getBoundingClientRect();return {{x:r.x+r.width/2,y:r.y+r.height/2}};}})()`);
 for(const type of ['mouseMoved','mousePressed','mouseReleased']) await page.call('Input.dispatchMouseEvent',{{type,...point,...(type==='mouseMoved'?{{}}:{{button:'left',clickCount:1}})}});
 if(await page.evaluate('window.testClicks')!==1) throw Error('Woken tab did not receive a click');
}} finally {{closeConnections();}}"""
                subprocess.run(['node', '--input-type=module', '-e', click], check=True, timeout=20)
            frame = origin.replace('127.0.0.1', 'localhost') + '/next'
            typing = f"""import {{ connect, closeConnections }} from './tests/native_cdp.mjs';
try {{
 const page=await connect(target=>target.url==={json.dumps(origin + '/')}, {app.content_port});
 await page.evaluate(`new Promise(r=>{{const frame=document.createElement('iframe');frame.src={json.dumps(frame)};frame.style='position:fixed;top:0;left:0;width:400px;height:300px';frame.onload=r;document.body.prepend(frame);}})`);
 await new Promise(r=>setTimeout(r,500));
 if(await page.evaluate('__felarisCanSleep()')!==true) throw Error('An idle cross-origin iframe blocked sleep');
 const child=await connect(target=>target.type==='iframe'&&target.url==={json.dumps(frame)}, {app.content_port});
 const point=await child.evaluate(`(()=>{{const r=document.getElementById('note').getBoundingClientRect();return {{x:r.x+r.width/2,y:r.y+r.height/2}};}})()`);
 for(const type of ['mouseMoved','mousePressed','mouseReleased']) await page.call('Input.dispatchMouseEvent',{{type,...point,...(type==='mouseMoved'?{{}}:{{button:'left',clickCount:1}})}});
 await page.call('Input.insertText',{{text:'iframe draft'}});
 if(await child.evaluate("document.getElementById('note').value")!=='iframe draft') throw Error('Typing did not reach the cross-origin iframe');
 await new Promise(r=>setTimeout(r,500));
 if(await page.evaluate('__felarisCanSleep()')!==false) throw Error('Typing in a cross-origin iframe did not block sleep');
}} finally {{closeConnections();}}"""
            subprocess.run(['node', '--input-type=module', '-e', typing], check=True, timeout=20)
            print(f'Tab sleep/wake releases native windows and preserves pointer input across six cycles; cross-origin iframe typing blocks sleep. Evidence: {app.artifacts}')
    finally:
        app.close()
        server.shutdown()
        server.server_close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/felaris.exe')
    check_tab_input(parser.parse_args().exe)
