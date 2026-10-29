"""An update restart must revalidate a restored page instead of retaining stale HTML."""
from http.server import ThreadingHTTPServer
import json
import subprocess
import threading
import time

from native_app import NativeApp
from native_smoke import Fixture


class CachedPage(Fixture):
    requests = []

    def do_GET(self):
        if self.path != '/cached':
            return super().do_GET()
        self.requests.append(self.headers.get('Cache-Control'))
        title = 'Cached blank page' if len(self.requests) == 1 else 'Fresh restored page'
        body = f'<html><head><title>{title}</title></head><body></body></html>'.encode()
        self.send_response(200)
        self.send_header('Content-Type', 'text/html')
        self.send_header('Cache-Control', 'max-age=3600')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def check_update_restore(executable):
    app = NativeApp(executable, 'update-restore')
    server = ThreadingHTTPServer(('127.0.0.1', 0), CachedPage)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    CachedPage.requests = []

    def sidebar(expression):
        result = subprocess.run(['node', 'tests/native_command.mjs', str(app.port), expression],
                                check=True, capture_output=True, text=True, timeout=20)
        return json.loads(result.stdout)

    def loaded(title):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if sidebar(f'currentState.tabs.find(tab=>tab.id===currentState.active_tab_id)?.title==={json.dumps(title)} && !currentState.is_loading'):
                return
            time.sleep(.1)
        raise AssertionError(f'Restored page did not load: {title}; requests: {CachedPage.requests}')

    def close():
        sidebar('window.ipc.postMessage(JSON.stringify({action:"window_close"}));true')
        app.process.wait(timeout=10)
        time.sleep(1)

    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([f'http://127.0.0.1:{server.server_port}/cached'], log)
            loaded('Cached blank page')
            close()
            app.start([], log)
            loaded('Cached blank page')
            assert len(CachedPage.requests) == 1, CachedPage.requests
            close()
            app.start(['--updated'], log)
            loaded('Fresh restored page')
            assert CachedPage.requests[-1] == 'no-cache', CachedPage.requests
            assert len(CachedPage.requests) == 2, CachedPage.requests
        print(f'Update restart revalidated cached blank HTML without manual reload. Evidence: {app.artifacts}')
    finally:
        app.close()
        server.shutdown()
        server.server_close()


if __name__ == '__main__':
    check_update_restore('target/debug/veluno.exe')
