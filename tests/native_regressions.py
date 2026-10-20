"""Drive the real sidebar IPC with WebView2 debugging in an isolated hidden app."""
import argparse
from http.server import ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import threading
import time
import uuid

from native_smoke import Fixture


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/felaris.exe')
    args = parser.parse_args()
    artifacts = (Path('target/native-regressions') / str(uuid.uuid4())).resolve()
    artifacts.mkdir(parents=True)
    profile = artifacts / 'profile'
    executable = artifacts / 'felaris.exe'
    shutil.copy2(Path(args.exe).resolve(), executable)
    with socket.socket() as reservation, socket.socket() as content_reservation:
        reservation.bind(('127.0.0.1', 0))
        content_reservation.bind(('127.0.0.1', 0))
        port = reservation.getsockname()[1]
        content_port = content_reservation.getsockname()[1]
    server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f'http://127.0.0.1:{server.server_port}'
    environment = {**os.environ, 'FELARIS_PROFILE_DIR': str(profile),
                   'FELARIS_SMOKE_TEST': str(artifacts / 'disabled.json'),
                   'FELARIS_SMOKE_BROWSER_ARGUMENTS': f'--remote-debugging-port={port} --remote-debugging-address=127.0.0.1',
                   'FELARIS_SMOKE_CONTENT_BROWSER_ARGUMENTS': f'--remote-debugging-port={content_port} --remote-debugging-address=127.0.0.1'}
    environment.pop('FELARIS_SMOKE_ORIGIN', None)
    checks = []
    process = None

    def state():
        try:
            return json.loads((profile / 'session.json').read_text(encoding='utf-8'))
        except (OSError, ValueError):
            return None

    def active(snapshot):
        return next(tab for tab in snapshot['tabs'] if tab['id'] == snapshot['active_tab_id'])

    def wait(predicate, description):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            snapshot = state()
            if snapshot and predicate(snapshot):
                return snapshot
            if process.poll() is not None:
                raise AssertionError(f'App exited before {description}')
            time.sleep(.1)
        raise AssertionError(f'Timed out: {description}. State: {state()}')

    def sidebar(expression):
        result = subprocess.run(['node', str(Path(__file__).with_name('native_command.mjs')), str(port), expression],
                                timeout=20, capture_output=True, text=True)
        if result.returncode:
            raise AssertionError(f'Native sidebar command failed:\n{result.stderr}')
        return json.loads(result.stdout)

    def command(action, **payload):
        message = json.dumps({'action': action, **payload})
        sidebar(f'window.ipc.postMessage({json.dumps(message)});true')

    def loaded(url):
        return wait(lambda snapshot: active(snapshot)['url'] == url and not active(snapshot)['is_loading'], f'load {url}')

    try:
        with (artifacts / 'app.log').open('w', encoding='utf-8') as log:
            process = subprocess.Popen([str(executable), origin + '/'], env=environment, stdout=log, stderr=log)
            command('navigate', url=origin + '/second')
            loaded(origin + '/second')
            command('navigate', url=origin)
            loaded(origin + '/')
            checks.append('An address without a trailing slash finishes loading')

            command('navigate', url=origin + '/spa')
            wait(lambda snapshot: active(snapshot)['url'] == origin + '/route' and active(snapshot)['title'] == 'Felaris route', 'pushState URL update')
            command('back')
            loaded(origin + '/')
            command('forward')
            loaded(origin + '/route')
            checks.append('pushState updates the saved URL and native Back and Forward still work')

            command('navigate', url='http://127.0.0.1:1/unreachable')
            wait(lambda snapshot: active(snapshot)['error'] and not active(snapshot)['is_loading'], 'native error page')
            command('back')
            recovered = loaded(origin + '/route')
            assert active(recovered)['error'] is None
            checks.append('Back leaves a failed navigation and clears its error')

            command('new_tab', url=origin + '/second')
            temporary = active(loaded(origin + '/second'))['id']
            command('sleep_inactive_tabs')
            command('close_tab', id=temporary)
            replacement = loaded(origin + '/route')
            assert not active(replacement)['is_sleeping']
            checks.append('Closing the active tab wakes its sleeping replacement')

            pinned = replacement['active_tab_id']
            command('toggle_pin', id=pinned)
            command('switch_workspace', id=2)
            other = wait(lambda snapshot: snapshot['active_workspace_id'] == 2, 'second workspace')['active_tab_id']
            command('switch_tab', id=pinned)
            command('new_tab', url=origin + '/slow')
            downloader = active(loaded(origin + '/slow'))['id']
            wait(lambda _: sidebar('currentState.active_downloads') == 1, 'active download')
            command('new_tab', url=origin + '/second')
            kept = active(loaded(origin + '/second'))['id']
            command('new_tab', url=origin + '/next')
            discard = active(loaded(origin + '/next'))['id']
            command('switch_tab', id=kept)
            command('clear_unpinned_tabs')
            remaining = wait(lambda snapshot: not any(tab['id'] == discard for tab in snapshot['tabs']), 'production Close Other Tabs')
            assert {tab['id'] for tab in remaining['tabs']} == {pinned, other, downloader, kept}
            assert remaining['active_tab_id'] == kept
            assert sidebar('currentState.active_downloads') == 1
            assert remaining['can_restore_closed_tab']
            command('restore_closed_tab')
            loaded(origin + '/next')
            checks.append('Close Other Tabs preserves pins, other workspaces, the active tab and a real download, with undo')
            Fixture.release_download.set()
            wait(lambda _: sidebar('currentState.active_downloads') == 0, 'download completion')
            assert (profile / 'smoke-download.txt').read_bytes() == b'x' * 65536
            subprocess.run(['node', str(Path(__file__).with_name('native_ui.mjs')), str(port), str(content_port), origin],
                           check=True, timeout=120)
            checks.append('Native keyboard interactions and 55 DOM checks at desktop and minimum sizes')
        (artifacts / 'results.json').write_text(json.dumps({'success': True, 'checks': checks}, indent=2), encoding='utf-8')
        print('\n'.join(checks))
        print(f'Headless regression tests passed. Evidence: {artifacts}')
    finally:
        Fixture.release_download.set()
        try:
            if sys.exc_info()[0] is not None and process is not None:
                print(f'Failed app PID {process.pid}, exit code {process.poll()}', flush=True)
                print(json.dumps((artifacts / 'app.log').read_text(encoding='utf-8', errors='replace')), flush=True)
                print(f'Saved fixture session: {json.dumps(state())}', flush=True)
                subprocess.run(['powershell', '-NoProfile', '-Command',
                                f'$parents = @({process.pid}); $all = @(Get-CimInstance Win32_Process); '
                                'do { $children = @($all | Where-Object { $_.ParentProcessId -in $parents -and $_.ProcessId -notin $parents }); '
                                '$parents += @($children.ProcessId) } while ($children.Count); '
                                '$all | Where-Object { $_.ProcessId -in $parents -and $_.CommandLine -notmatch " --type=" } | Select-Object ProcessId,ParentProcessId,Name,CommandLine | Format-List'],
                               timeout=15)
        finally:
            if process is not None and process.poll() is None:
                process.kill()
                process.wait(timeout=10)
            executable.unlink(missing_ok=True)
            server.shutdown()



if __name__ == '__main__':
    main()
