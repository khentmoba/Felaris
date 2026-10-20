"""Drive the real sidebar IPC with WebView2 debugging in an isolated hidden app."""
import argparse
import copy
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
from urllib.request import urlopen

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
                                timeout=20, capture_output=True, text=True, encoding='utf-8')
        if result.returncode:
            raise AssertionError(f'Native sidebar command failed:\n{result.stderr}')
        return json.loads(result.stdout)

    def command(action, **payload):
        message = json.dumps({'action': action, **payload})
        sidebar(f'window.ipc.postMessage({json.dumps(message)});true')

    def content(url, expression):
        result = subprocess.run(['node', str(Path(__file__).with_name('native_command.mjs')), str(content_port), expression, url],
                                timeout=20, capture_output=True, text=True, encoding='utf-8')
        if result.returncode:
            raise AssertionError(f'Content command failed:\n{result.stderr}')
        return json.loads(result.stdout)

    def switch(id):
        return sidebar(f'''(async()=>{{
            const started=performance.now();
            window.ipc.postMessage(JSON.stringify({{action:'switch_tab',id:{id}}}));
            while(performance.now()-started<10000) {{
                const tab=currentState.tabs.find(tab=>tab.id==={id});
                if(currentState.active_tab_id==={id} && !tab.is_loading && !tab.is_sleeping) return performance.now()-started;
                await new Promise(resolve=>setTimeout(resolve,20));
            }}
            throw Error('Tab {id} did not wake');
        }})()''')

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
            checks.append('Native keyboard interactions and 56 DOM checks at desktop and minimum sizes')

            source = next(tab['id'] for tab in sidebar('currentState.tabs') if tab['url'] == origin + '/next')
            command('duplicate_tab', id=source)
            duplicate = loaded(origin + '/next')['active_tab_id']
            assert duplicate != source
            command('move_tab_to_workspace', id=duplicate, workspace_id=2)
            wait(lambda snapshot: any(tab['id'] == duplicate and tab['workspace_id'] == 2 for tab in snapshot['tabs']),
                 'moved duplicate reaches disk')
            command('close_tab', id=duplicate)
            wait(lambda snapshot: all(tab['id'] != duplicate for tab in snapshot['tabs']), 'duplicate closes')
            command('expand_popup', url=origin + '/expanded')
            expanded = loaded(origin + '/expanded')['active_tab_id']
            command('close_tab', id=expanded)
            wait(lambda snapshot: all(tab['id'] != expanded for tab in snapshot['tabs']), 'expanded tab closes')
            checks.append('Duplicated tabs, workspace moves, and popup expansion persist through the session writer')

            seed = sidebar('currentState')
            capacity = []
            for count in (100, 300, 500):
                process.kill()
                process.wait(timeout=10)
                time.sleep(1)
                snapshot = copy.deepcopy(seed)
                template = snapshot['tabs'][0]
                snapshot['tabs'] = [{**template, 'id': id, 'workspace_id': 1, 'title': f'Capacity {id}',
                                     'url': f'{origin}/capacity/{id}', 'is_pinned': id == 1,
                                     'is_loading': False, 'error': None, 'is_sleeping': True}
                                    for id in range(1, count + 1)]
                snapshot.update(active_tab_id=count, active_workspace_id=1, closed_tabs=[], active_downloads=0)
                (profile / 'session.json').write_text(json.dumps(snapshot), encoding='utf-8')
                Fixture.release_download.clear()
                started = time.monotonic()
                process = subprocess.Popen([str(executable)], env=environment, stdout=log, stderr=log)
                wait(lambda _: sidebar('currentState.tabs.length') == count, 'capacity sidebar ready')
                loaded(f'{origin}/capacity/{count}')
                startup_ms = (time.monotonic() - started) * 1000
                with urlopen(f'http://127.0.0.1:{content_port}/json/list', timeout=5) as response:
                    pages = json.load(response)
                assert len([page for page in pages if page['type'] == 'page']) == 1, 'restore must create only the active WebView'
                latencies = [switch(1), switch(2)]
                content(f'{origin}/capacity/2', "document.getElementById('note').value='protected draft';true")
                latencies.append(switch(3))
                assert content(f'{origin}/capacity/3', f"(async()=>{{window.media=new Audio('{origin}/tone.wav');media.muted=true;media.loop=true;document.body.append(media);await media.play();return !media.paused}})()")
                content(f'{origin}/capacity/3', 'window.mediaMarker="retained playback";true')
                latencies.append(switch(4))
                assert not content(f'{origin}/capacity/3', '__felarisCanSleep()'), 'hidden playback must stay protected even if WebView2 pauses it'
                command('navigate', url=origin + '/slow')
                loaded(origin + '/slow')
                wait(lambda snapshot: sidebar('currentState.active_downloads') == 1, 'capacity download starts')
                latencies.extend(switch(id) for id in range(5, 31))
                settled = wait(lambda snapshot: len(snapshot['tabs']) == count and sum(not tab['is_sleeping'] for tab in snapshot['tabs']) <= 12,
                               f'{count} tabs settle to the live budget')
                for id in (1, 2, 3, 4):
                    assert not next(tab for tab in settled['tabs'] if tab['id'] == id)['is_sleeping'], f'protected tab {id} slept'
                assert content(f'{origin}/capacity/2', "document.getElementById('note').value") == 'protected draft'
                assert content(f'{origin}/capacity/3', 'window.mediaMarker') == 'retained playback'
                with urlopen(f'http://127.0.0.1:{content_port}/json/list', timeout=5) as response:
                    live_pages = len([page for page in json.load(response) if page['type'] == 'page'])
                assert live_pages <= 12, f'WebViews leaked: {live_pages}'
                memory = subprocess.run(['powershell', '-NoProfile', '-Command',
                                        f'$ids=@({process.pid}); $all=@(Get-CimInstance Win32_Process); '
                                        'do { $children=@($all | Where-Object { $_.ParentProcessId -in $ids -and $_.ProcessId -notin $ids }); '
                                        '$ids+=@($children.ProcessId) } while($children.Count); '
                                        '($all | Where-Object { $_.ProcessId -in $ids } | Measure-Object WorkingSetSize -Sum).Sum'],
                                       check=True, capture_output=True, text=True, timeout=20)
                capacity.append({'tabs': count, 'live_views': live_pages, 'startup_ms': round(startup_ms),
                                 'max_switch_ms': round(max(latencies)), 'working_set_bytes': int(memory.stdout.strip())})
                Fixture.release_download.set()
                wait(lambda snapshot: sidebar('currentState.active_downloads') == 0, 'capacity download finishes')
                content(f'{origin}/capacity/3', 'media.pause();true')
                sleeper = next(tab['id'] for tab in settled['tabs'] if tab['is_sleeping'])
                switch(sleeper)
                wait(lambda snapshot: snapshot['active_tab_id'] == sleeper, 'latest selection reaches disk')
                process.kill()
                process.wait(timeout=10)
                time.sleep(1)
                process = subprocess.Popen([str(executable)], env=environment, stdout=log, stderr=log)
                wait(lambda _: sidebar('currentState.tabs.length') == count, 'recovered capacity sidebar ready')
                recovered = wait(lambda snapshot: snapshot['active_tab_id'] == sleeper and not active(snapshot)['is_loading'], 'capacity crash recovery')
                assert sum(not tab['is_sleeping'] for tab in recovered['tabs']) == 1
                assert next(tab for tab in recovered['tabs'] if tab['id'] == 1)['is_pinned']
                print(f'Capacity {count}: {capacity[-1]}', flush=True)
            (artifacts / 'capacity.json').write_text(json.dumps(capacity, indent=2), encoding='utf-8')
            checks.append('100/300/500-tab sessions restore lazily, bound actual WebViews, protect drafts/media/pins/downloads, wake sleepers, and recover after forced exit')
        (artifacts / 'results.json').write_text(json.dumps({'success': True, 'checks': checks}, indent=2), encoding='utf-8')
        print('\n'.join(checks))
        print(f'Headless regression tests passed. Evidence: {artifacts}')
    finally:
        Fixture.release_download.set()
        try:
            if sys.exc_info()[0] is not None and process is not None:
                print(f'Failed app PID {process.pid}, exit code {process.poll()}', flush=True)
                print(json.dumps((artifacts / 'app.log').read_text(encoding='utf-8', errors='replace')), flush=True)
                snapshot = state()
                print(f'Saved fixture session: {json.dumps({"tabs": len(snapshot["tabs"]), "active": active(snapshot)}) if snapshot else None}', flush=True)
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
