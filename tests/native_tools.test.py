"""Prove capture ownership and failure evidence with faults in the real UI driver."""
import argparse
from http.server import ThreadingHTTPServer
import json
from pathlib import Path
import subprocess
import sys
import threading
import time

from native_app import NativeApp
from native_smoke import Fixture


class DelayedKeyboardFixture(Fixture):
    def do_GET(self):
        if self.path in ('/keyboard', '/submitted'):
            time.sleep(.5)
        super().do_GET()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/felaris.exe')
    args = parser.parse_args()
    app = NativeApp(args.exe, 'native-tools')
    server = ThreadingHTTPServer(('127.0.0.1', 0), DelayedKeyboardFixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f'http://127.0.0.1:{server.server_port}'
    tests = Path(__file__).resolve().parent

    def evaluate(expression):
        result = subprocess.run(['node', str(tests / 'native_command.mjs'), str(app.port), expression],
                                check=True, capture_output=True, text=True, encoding='utf-8', timeout=20)
        return json.loads(result.stdout)

    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([origin], log)
            evaluate('currentState.sidebar_collapsed')
            subprocess.run([sys.executable, '-B', str(tests / 'native_capture.py'), '--exe', args.exe],
                           check=True, timeout=60)
            assert app.process.poll() is None, 'Capturing another app terminated the decoy'
            assert evaluate('currentState.sidebar_collapsed') is False, 'Decoy stopped responding'
            print('Owned capture cleanup preserves a separate running Felaris instance.', flush=True)

            source = (tests / 'native_ui.mjs').read_text(encoding='utf-8')
            source = source.replace("from './native_cdp.mjs'", f'from {json.dumps((tests / "native_cdp.mjs").as_uri())}')
            menu_marker = '  const menuOpacity = await sidebar.evaluate('
            key_marker = "  await press(restored, 'B', 10);"
            command_marker = '  const command = '
            sidebar_marker = "  const sidebar = await connect(page => page.title === 'Felaris');"
            assert source.count(menu_marker) == source.count(key_marker) == source.count(command_marker) == source.count(sidebar_marker) == 1
            menu_source = source.replace(sidebar_marker, sidebar_marker + "\n  await sidebar.call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });")
            menu_source = menu_source.replace(command_marker, "  throw Error('Injected menu fade was not rejected');\n" + command_marker)
            faults = [
                ('menu', menu_source.replace(menu_marker, '''  await sidebar.evaluate(`(() => {
    const style = document.createElement('style'); style.id = 'retro-fault';
    style.textContent = '@keyframes menu-in { from { opacity:.6; } to { opacity:1; } }';
    document.head.append(style); return true;
  })()`);
''' + menu_marker), 'App menu must stay opaque'),
                ('keyboard', source.replace(key_marker, ''), 'Timed out: Ctrl+Shift+B collapses sidebar'),
            ]
            for name, driver, expected in faults:
                probe = app.artifacts / f'{name}.mjs'
                probe.write_text(driver, encoding='utf-8')
                result = subprocess.run(['node', str(probe), str(app.port), str(app.content_port), origin, str(app.artifacts)],
                                        capture_output=True, text=True, encoding='utf-8', timeout=60)
                (app.artifacts / f'{name}.log').write_text(result.stdout + result.stderr, encoding='utf-8')
                assert result.returncode != 0 and expected in result.stderr, result.stderr
                report = json.loads((app.artifacts / 'debugger.json').read_text(encoding='utf-8'))
                assert expected in report['error'], report
                sidebar = next(target for target in report['targets'] if target.get('title') == 'Felaris')
                assert sidebar['state']['ready'] == 'complete', report
                if name == 'menu':
                    assert sidebar['state']['reducedMotion'] is False, report
                if name == 'keyboard':
                    assert report['inputs'] and sidebar['state']['sidebarCollapsed'] is False, report
                (app.artifacts / f'{name}-failure.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
                evaluate("document.getElementById('retro-fault')?.remove(); true")
                print(f'{name}: deliberate regression rejected, focused evidence retained.', flush=True)
        print(f'Native tooling checks passed. Evidence: {app.artifacts}')
    finally:
        try:
            app.close()
        finally:
            server.shutdown()


if __name__ == '__main__':
    main()
