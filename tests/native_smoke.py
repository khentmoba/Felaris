"""Exercise the shipped Rust/WebView2 app against local fixtures and an isolated profile."""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import threading
import time
import uuid


class Fixture(BaseHTTPRequestHandler):
    release_download = threading.Event()

    def do_GET(self):
        path = self.path.split('?', 1)[0]
        if path in ('/tests/ui.html', '/src/ui.html', '/src/start_page.html', '/src/new_tab.html', '/src/browser.js'):
            body = (Path(__file__).resolve().parent.parent / path.lstrip('/')).read_bytes()
            self.send_response(200)
            self.send_header('Content-Type', 'text/javascript; charset=utf-8' if path.endswith('.js') else 'text/html; charset=utf-8')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        if self.path in ('/download', '/slow-download'):
            body = b'x' * 65536 if self.path == '/slow-download' else b'Felaris download fixture\n'
            self.send_response(200)
            self.send_header('Content-Type', 'application/octet-stream')
            self.send_header('Content-Disposition', 'attachment; filename="felaris-smoke.txt"')
        else:
            title = {'/': 'fixture', '/second': 'second', '/next': 'next', '/popup': 'popup'}.get(self.path, 'fixture')
            extra = "<script>setTimeout(()=>{history.pushState({},'', '/route');document.title='Felaris route'},300)</script>" if self.path == '/spa' else ''
            if self.path == '/slow':
                extra = "<script>setTimeout(()=>document.getElementById('download').click(),500)</script>"
            download = '/slow-download' if self.path == '/slow' else '/download'
            body = f'''<!doctype html><html><meta charset="utf-8"><title>Felaris {title}</title>
<body style="font:16px system-ui;padding:40px"><h1>Felaris {title}</h1>
<label>Keep this draft <input id="note"></label><p><a href="/next">Next page</a></p>
<p><a href="/popup" target="_blank">Open a new tab</a></p>
<a id="download" href="{download}">Download test file</a>{extra}</body></html>'''.encode()
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        if self.path == '/slow-download':
            self.wfile.write(body[:32768])
            self.wfile.flush()
            self.release_download.wait(timeout=20)
            self.wfile.write(body[32768:])
        else:
            self.wfile.write(body)

    def log_message(self, *_):
        pass


def await_report(process, path):
    deadline = time.monotonic() + 45
    while time.monotonic() < deadline:
        if path.exists():
            result = json.loads(path.read_text(encoding='utf-8'))
            print(json.dumps(result))
            if not result['success']:
                raise AssertionError(result)
            return
        if process.poll() is not None:
            raise AssertionError(f'Felaris exited with {process.returncode} before writing {path}')
        time.sleep(.1)
    raise AssertionError(f'Native test timed out; see {path.parent}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/felaris.exe')
    args = parser.parse_args()
    executable = Path(args.exe).resolve()
    artifacts = Path('target/native-smoke') / str(uuid.uuid4())
    artifacts.mkdir(parents=True)
    artifacts = artifacts.resolve()
    server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f'http://127.0.0.1:{server.server_port}'
    environment = {**os.environ, 'FELARIS_PROFILE_DIR': str(artifacts / 'profile'),
                   'FELARIS_SMOKE_ORIGIN': origin, 'FELARIS_SMOKE_TEST': str(artifacts / 'first.json'),
                   'FELARIS_SMOKE_HOLD': '1'}
    process = None
    try:
        with (artifacts / 'first.log').open('w') as log:
            process = subprocess.Popen([str(executable), origin + '/'], env=environment, stdout=log, stderr=log)
            await_report(process, artifacts / 'first.json')
            session = json.loads((artifacts / 'profile/session.json').read_text(encoding='utf-8'))
            assert session['active_workspace_id'] == 2
            assert any(tab['is_pinned'] for tab in session['tabs'])
            secondary = {key: value for key, value in environment.items() if not key.startswith('FELARIS_SMOKE_')}
            secondary['FELARIS_SMOKE_TEST'] = environment['FELARIS_SMOKE_TEST']
            assert subprocess.run([str(executable), origin + '/second'], env=secondary, timeout=15).returncode == 0
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                updated = json.loads((artifacts / 'profile/session.json').read_text(encoding='utf-8'))
                if len(updated['tabs']) == len(session['tabs']) + 1:
                    break
                time.sleep(.1)
            else:
                raise AssertionError('Second launch did not hand its link to the running instance')

            process.kill()
            process.wait(timeout=10)
        time.sleep(1)
        environment.pop('FELARIS_SMOKE_HOLD')
        environment['FELARIS_SMOKE_TEST'] = str(artifacts / 'restored.json')
        environment['FELARIS_SMOKE_RESTORE'] = '1'
        with (artifacts / 'restored.log').open('w') as log:
            process = subprocess.Popen([str(executable)], env=environment, stdout=log, stderr=log)
            await_report(process, artifacts / 'restored.json')
            assert process.wait(timeout=10) == 0
        (artifacts / 'profile/session.json').write_text('{truncated', encoding='utf-8')
        environment['FELARIS_SMOKE_TEST'] = str(artifacts / 'recovered.json')
        with (artifacts / 'recovered.log').open('w') as log:
            process = subprocess.Popen([str(executable)], env=environment, stdout=log, stderr=log)
            await_report(process, artifacts / 'recovered.json')
            assert process.wait(timeout=10) == 0
        print(f'Headless native smoke tests passed, including link handoff and corrupt-session backup recovery. Evidence: {artifacts}')
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        server.shutdown()


if __name__ == '__main__':
    main()
