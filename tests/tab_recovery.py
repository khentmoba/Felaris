"""A crashed tab should recover when selected again without a manual reload."""
from http.server import ThreadingHTTPServer
import json
import subprocess
import threading
import time

from native_app import NativeApp
from native_smoke import Fixture


def check_recovery(executable):
    app = NativeApp(executable, 'tab-recovery')
    server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    origin = f'http://127.0.0.1:{server.server_port}'
    threading.Thread(target=server.serve_forever, daemon=True).start()

    def sidebar(expression):
        result = subprocess.run(['node', 'tests/native_command.mjs', str(app.port), expression],
                                check=True, capture_output=True, text=True, timeout=20)
        return json.loads(result.stdout)

    def wait(expression):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if sidebar(expression):
                return
            time.sleep(.1)
        raise AssertionError(f'Tab did not recover automatically: {expression}')

    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([origin], log)
            wait('currentState.tabs.find(tab=>tab.id===currentState.active_tab_id)?.title==="Felaris fixture" && !currentState.is_loading')
            crash = f"""import {{ connect, closeConnections }} from './tests/native_cdp.mjs';
const view = await connect(page=>page.url.startsWith({json.dumps(origin)}), {app.content_port});
try {{ await view.call('Page.crash', {{}}, {{allowTargetClose:true}}); }} catch (error) {{
 if(!/crash|WebView closed/i.test(error.message)) throw error;
}} finally {{ closeConnections(); }}"""
            subprocess.run(['node', '--input-type=module', '-e', crash], check=True, timeout=20)
            wait('currentState.tabs.some(tab=>tab.error && !tab.is_loading)')
            failed = sidebar('currentState.active_tab_id')
            sidebar('window.ipc.postMessage(JSON.stringify({action:"new_tab",url:"felaris://newtab"}));true')
            wait(f'currentState.active_tab_id!=={failed}')
            sidebar(f'window.ipc.postMessage(JSON.stringify({{action:"switch_tab",id:{failed}}}));true')
            wait('currentState.tabs.find(tab=>tab.id===currentState.active_tab_id)?.title==="Felaris fixture" && !currentState.tabs.find(tab=>tab.id===currentState.active_tab_id)?.error && !currentState.is_loading')
        print(f'Crashed tab recovered on selection without manual reload. Evidence: {app.artifacts}')
    finally:
        app.close()
        server.shutdown()
        server.server_close()


if __name__ == '__main__':
    check_recovery('target/debug/felaris.exe')
