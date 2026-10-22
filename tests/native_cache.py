"""Reproduce a failed service-worker bootstrap and exercise the browser recovery path."""
import argparse
from http.server import ThreadingHTTPServer
import subprocess
import threading

from native_app import NativeApp
from native_smoke import Fixture


def check_cache_recovery(executable):
    app = NativeApp(executable, 'native-cache')
    server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f'http://127.0.0.1:{server.server_port}'
    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([origin], log)
            subprocess.run(['node', 'tests/native_cache.mjs', str(app.port), str(app.content_port), origin, str(app.artifacts)],
                           check=True, timeout=90)
    finally:
        app.close()
        server.shutdown()
        server.server_close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/felaris.exe')
    check_cache_recovery(parser.parse_args().exe)
