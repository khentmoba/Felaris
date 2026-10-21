"""Exercise download-only tab cleanup in an owned, isolated WebView2 app."""
import argparse
from http.server import ThreadingHTTPServer
from pathlib import Path
import subprocess
import threading

from native_app import NativeApp
from native_smoke import Fixture


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/felaris.exe')
    args = parser.parse_args()
    app = NativeApp(args.exe, 'native-downloads')
    server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f'http://127.0.0.1:{server.server_port}'
    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([origin], log)
            subprocess.run(['node', str(Path(__file__).with_suffix('.mjs')),
                            str(app.port), str(app.content_port), origin, str(app.artifacts)],
                           check=True, timeout=90)
        print(f'Native download checks passed. Evidence: {app.artifacts}')
    finally:
        try:
            app.close()
        finally:
            Fixture.release_download.set()
            server.shutdown()


if __name__ == '__main__':
    main()
