"""Measure private working set and commit of an owned Veluno process tree while opening cross-site tabs.

Manual benchmark (needs psutil); not part of CI: python -B tests/native_memory.py --exe <exe> --tabs 30 --idle 120
"""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import statistics
import subprocess
import threading
import time

import psutil

from native_app import NativeApp

# Each tab is its own site (*.localhost resolves to loopback), so it gets its own renderer like real browsing.
PAGE = b'''<!doctype html><meta charset="utf-8"><title>Memory fixture</title>
<body style="font:16px system-ui"><canvas id="c" width="1200" height="800"></canvas><div id="list"></div>
<script>
window.heap = Array.from({length: 200000}, (_, i) => ({i, text: 'row ' + i}));
const list = document.getElementById('list');
for (let i = 0; i < 3000; i++) { const p = document.createElement('p'); p.textContent = 'Paragraph ' + i; list.append(p); }
const g = document.getElementById('c').getContext('2d');
for (let i = 0; i < 400; i++) { g.fillStyle = `hsl(${i} 70% 50%)`; g.fillRect(i * 3, i * 2, 200, 120); }
document.addEventListener('visibilitychange', () => {
  if (document.hidden) return;
  const shown = performance.now();
  requestAnimationFrame(() => requestAnimationFrame(() => { window.paintMs = performance.now() - shown; }));
});
</script>'''


class Page(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/html; charset=utf-8')
        self.send_header('Content-Length', str(len(PAGE)))
        self.end_headers()
        self.wfile.write(PAGE)

    def log_message(self, *_):
        pass


def footprint(pid):
    root = psutil.Process(pid)
    total, commit, renderers, processes, kinds = 0, 0, 0, 0, {}
    for process in [root, *root.children(recursive=True)]:
        try:
            memory = process.memory_full_info()
            total += memory.uss
            commit += memory.private
            processes += 1
            command = ' '.join(process.cmdline())
            renderers += '--type=renderer' in command
            kind = ('ui ' if r'\Chrome\EBWebView' in command else 'tab ') + (command.split('--type=')[1].split()[0] if '--type=' in command else 'browser')
            kinds[kind] = kinds.get(kind, 0) + memory.uss // 2**20
        except psutil.NoSuchProcess:
            pass
    return {'private_mib': round(total / 2**20), 'commit_mib': round(commit / 2**20), 'processes': processes, 'renderers': renderers, 'mib_by_kind': kinds}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/release/veluno.exe')
    parser.add_argument('--tabs', type=int, default=30)
    parser.add_argument('--idle', type=int, default=60, help='seconds to idle before the final sample')
    parser.add_argument('--label', default='run')
    parser.add_argument('--chrome-args', default='', help='extra browser arguments for the UI environment')
    parser.add_argument('--content-args', default='', help='extra browser arguments for the tab environment')
    args = parser.parse_args()
    app = NativeApp(args.exe, 'native-memory')
    app.environment['VELUNO_SMOKE_BROWSER_ARGUMENTS'] += ' ' + args.chrome_args
    app.environment['VELUNO_SMOKE_CONTENT_BROWSER_ARGUMENTS'] += ' ' + args.content_args
    server = ThreadingHTTPServer(('127.0.0.1', 0), Page)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    port = server.server_port

    def sidebar(expression):
        result = subprocess.run(['node', 'tests/native_command.mjs', str(app.port), expression],
                                check=True, capture_output=True, text=True, timeout=30)
        return json.loads(result.stdout)

    def open_tab(url):
        return sidebar(f'''(async()=>{{
            const started=performance.now();
            window.ipc.postMessage(JSON.stringify({{action:'new_tab',url:{json.dumps(url)}}}));
            while(performance.now()-started<15000) {{
                const tab=currentState.tabs.find(tab=>tab.id===currentState.active_tab_id);
                if(tab.url==={json.dumps(url)} && !tab.is_loading) return performance.now()-started;
                await new Promise(resolve=>setTimeout(resolve,10));
            }}
            throw Error('Tab did not load');
        }})()''')

    def content(url, expression):
        result = subprocess.run(['node', 'tests/native_command.mjs', str(app.content_port), expression, url],
                                check=True, capture_output=True, text=True, timeout=30)
        return json.loads(result.stdout)

    def switch(id):
        return sidebar(f'''(async()=>{{
            const started=performance.now();
            window.ipc.postMessage(JSON.stringify({{action:'switch_tab',id:{id}}}));
            while(performance.now()-started<15000) {{
                const tab=currentState.tabs.find(tab=>tab.id==={id});
                if(currentState.active_tab_id==={id} && !tab.is_loading && !tab.is_sleeping) return performance.now()-started;
                await new Promise(resolve=>setTimeout(resolve,10));
            }}
            throw Error('Tab did not wake');
        }})()''')

    samples, loads = {}, []
    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            process = app.start([f'http://s0.localhost:{port}/'], log)
            sidebar('currentState.tabs.length')
            time.sleep(5)
            samples['1 tab'] = footprint(process.pid)
            for index in range(1, args.tabs + 1):
                loads.append(open_tab(f'http://s{index}.localhost:{port}/'))
                if index in (10, args.tabs):
                    time.sleep(5)
                    samples[f'{index} tabs'] = footprint(process.pid)
            for second in range(0, args.idle, 30):
                time.sleep(30)
                print(second + 30, footprint(process.pid), flush=True)
            samples[f'{args.tabs} tabs idle {args.idle}s'] = footprint(process.pid)
            tabs = [tab for tab in sidebar('currentState.tabs') if '.localhost:' in tab['url']]
            ids = [tab['id'] for tab in tabs]
            switches = [switch(id) for id in ids[-12:-1]]  # recent hidden tabs, normally still live
            time.sleep(1)
            # Tabs that slept reload on wake and report no paint.
            paints = [paint for tab in tabs[-12:-1] if (paint := content(tab['url'], 'window.paintMs ?? null')) is not None]
            wakes = [switch(id) for id in ids[1:6]]  # oldest tabs, asleep once past the live budget
            samples['after switching'] = footprint(process.pid)
            live = sum(not tab['is_sleeping'] for tab in sidebar('currentState.tabs'))
    finally:
        app.close()
        server.shutdown()
    result = {'label': args.label, 'samples': samples, 'live_tabs': live,
              'load_ms_median': round(statistics.median(loads)), 'load_ms_max': round(max(loads)),
              'switch_ms_median': round(statistics.median(switches)), 'paint_ms_median': round(statistics.median(paints), 1) if paints else None, 'paint_ms_max': round(max(paints), 1) if paints else None, 'wake_ms_median': round(statistics.median(wakes))}
    (app.artifacts / 'memory.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
