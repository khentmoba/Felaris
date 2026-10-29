"""Capture local fixture WebViews without opening or closing the user's browser."""
import argparse
from http.server import ThreadingHTTPServer
from pathlib import Path
import subprocess
import threading

from native_app import NativeApp
from native_smoke import Fixture


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/veluno.exe')
    args = parser.parse_args()
    app = NativeApp(args.exe, 'native-capture')
    server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f'http://127.0.0.1:{server.server_port}'
    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([origin], log)
            subprocess.run(['node', str(Path(__file__).with_name('native_capture.mjs')),
                            str(app.port), str(app.content_port), str(app.artifacts)], check=True, timeout=60)
        print(f'Fixture WebView captures: {app.artifacts}')
    finally:
        try:
            app.close()
        finally:
            server.shutdown()


if __name__ == '__main__':
    main()
