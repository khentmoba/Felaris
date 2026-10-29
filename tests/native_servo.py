"""Headless runtime checks for the Servo engine (`--features servo`, `--engine=servo`).

Each check starts an isolated, hidden Felaris with a scratch profile, drives it through a local
test server and the WebView2 sidebar's debugger, and reads results from the app's event log.
Native dialogs run on a private Windows desktop that is never shown to the user.
"""
import argparse
import ctypes
import ctypes.wintypes as wt
import http.server
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import threading
import time
import uuid

REPO = Path(__file__).resolve().parent.parent
user32 = ctypes.WinDLL('user32', use_last_error=True)
ENUM = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
user32.CreateDesktopW.argtypes = [wt.LPCWSTR, wt.LPCWSTR, ctypes.c_void_p, wt.DWORD, wt.DWORD, ctypes.c_void_p]
user32.CreateDesktopW.restype = wt.HANDLE
user32.CloseDesktop.argtypes = [wt.HANDLE]
user32.GetProcessWindowStation.restype = wt.HANDLE
user32.GetThreadDesktop.argtypes = [wt.DWORD]
user32.GetThreadDesktop.restype = wt.HANDLE
user32.GetUserObjectInformationW.argtypes = [wt.HANDLE, ctypes.c_int, ctypes.c_void_p, wt.DWORD, ctypes.POINTER(wt.DWORD)]
user32.EnumDesktopWindows.argtypes = [wt.HANDLE, ENUM, wt.LPARAM]
user32.GetWindowThreadProcessId.argtypes = [wt.HWND, ctypes.POINTER(wt.DWORD)]
user32.GetClassNameW.argtypes = [wt.HWND, wt.LPWSTR, ctypes.c_int]
user32.GetDlgItem.argtypes = [wt.HWND, ctypes.c_int]
user32.GetDlgItem.restype = wt.HWND
user32.SendMessageW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
user32.PostMessageW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
user32.SetWindowPos.argtypes = [wt.HWND, wt.HWND, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wt.UINT]
DESKTOP = None
SLOW_CHUNKS = 40
RUN_PREFIX = 'felaris-servo-' + uuid.uuid4().hex
RESULTS = []


class MemoryCounters(ctypes.Structure):
    _fields_ = [('size', wt.DWORD), ('faults', wt.DWORD)] + [
        (name, ctypes.c_size_t) for name in ('peak', 'working', 'peak_paged', 'paged',
                                           'peak_nonpaged', 'nonpaged', 'pagefile', 'peak_pagefile')]


def peak_memory(pid):
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    psapi = ctypes.WinDLL('psapi', use_last_error=True)
    kernel.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
    kernel.OpenProcess.restype = wt.HANDLE
    kernel.CloseHandle.argtypes = [wt.HANDLE]
    psapi.GetProcessMemoryInfo.argtypes = [wt.HANDLE, ctypes.POINTER(MemoryCounters), wt.DWORD]
    handle = kernel.OpenProcess(0x410, False, pid)
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        counters = MemoryCounters()
        counters.size = ctypes.sizeof(counters)
        if not psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.size):
            raise ctypes.WinError(ctypes.get_last_error())
        return counters.peak
    finally:
        kernel.CloseHandle(handle)


def downloads_folder():
    out = subprocess.run(['powershell', '-NoProfile', '-Command',
                          "(New-Object -ComObject Shell.Application).Namespace('shell:Downloads').Self.Path"],
                         capture_output=True, text=True, check=True).stdout.strip()
    return Path(out)


def dialogs(pid):
    found = []

    def visit(hwnd, _):
        owner = wt.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        name = ctypes.create_unicode_buffer(64)
        user32.GetClassNameW(hwnd, name, 64)
        if owner.value == pid and name.value == '#32770':
            found.append(hwnd)
        return True

    user32.EnumDesktopWindows(DESKTOP, ENUM(visit), 0)
    return found


def answer_dialog(pid, command, fields=None, timeout=15):
    """Waits for a native dialog in `pid`, moves it off screen, fills edits, sends `command`."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        for dialog in dialogs(pid):
            user32.SetWindowPos(dialog, None, -32000, -32000, 0, 0, 0x15)
            time.sleep(0.3)
            for item, text in (fields or {}).items():
                value = ctypes.create_unicode_buffer(text)
                user32.SendMessageW(user32.GetDlgItem(dialog, item), 0x000C, 0, ctypes.addressof(value))
            time.sleep(0.2)
            user32.PostMessageW(dialog, 0x0111, command, 0)
            return True
        time.sleep(0.05)
    return False


class Handler(http.server.BaseHTTPRequestHandler):
    hits = {}

    def log_message(self, *args):
        pass

    def send(self, status, headers, body):
        self.send_response(status)
        for name, value in headers:
            self.send_header(name, value)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        host = self.headers.get('Host', '').split(':')[0]
        Handler.hits[(host, self.path)] = Handler.hits.get((host, self.path), 0) + 1
        port = self.server.server_port
        if self.path in ('/slow', '/stall'):
            self.send_response(200)
            self.send_header('Content-Type', 'application/octet-stream')
            name = 'stall' if self.path == '/stall' else 'slow'
            self.send_header('Content-Disposition', f'attachment; filename="{RUN_PREFIX}-{name}.bin"')
            self.send_header('Content-Length', str(SLOW_CHUNKS * 1000))
            self.end_headers()
            if self.path == '/stall':
                time.sleep(30)
                return
            try:
                for _ in range(SLOW_CHUNKS):
                    self.wfile.write(b'x' * 1000)
                    self.wfile.flush()
                    time.sleep(0.5)
            except OSError:
                pass
        elif self.path == '/large':
            self.send_response(200)
            self.send_header('Content-Type', 'application/octet-stream')
            self.send_header('Content-Disposition', f'attachment; filename="{RUN_PREFIX}-large.bin"')
            self.send_header('Content-Length', str(64 * 1024 * 1024))
            self.end_headers()
            try:
                for _ in range(1024):
                    self.wfile.write(b'x' * 65536)
                    self.wfile.flush()
                    time.sleep(0.003)
            except OSError:
                pass
        elif self.path == '/attach':
            self.send(200, [('Content-Type', 'text/plain'),
                            ('Content-Disposition', f"attachment; filename=\"x.txt\"; filename*=UTF-8''{RUN_PREFIX}-na%C3%AFve.txt")],
                      b'payload\n' * 1000)
        elif self.path == '/cache':
            body = (f"<!doctype html><title>cache</title>"
                    f"<script src='http://127.0.0.1:{port}/same.js'></script>"
                    f"<script src='http://127.0.0.2:{port}/other.js'></script>").encode()
            self.send(200, [('Content-Type', 'text/html'), ('Cache-Control', 'no-store')], body)
        elif self.path.endswith('.js'):
            self.send(200, [('Content-Type', 'text/javascript'), ('Cache-Control', 'max-age=3600')], b'void 0;')
        elif self.path.startswith('/?') or self.path == '/':
            self.send(200, [('Content-Type', 'text/html')], PAGE)
        else:
            self.send(404, [('Content-Type', 'text/plain')], b'not found')


PAGE = b"""<!doctype html><title>start</title>
<input type=color id=c value=#000000>
<script>
const q = new URLSearchParams(location.search);
const c = document.getElementById('c');
c.addEventListener('input', () => document.title = 'input:' + c.value);
c.addEventListener('change', () => document.title += ' change:' + c.value);
if (q.get('go')) setTimeout(() => { document.title = 'leaving'; location.href = '/' + q.get('go'); }, 1500);
const storage = q.get('storage');
if (storage) {
  const before = localStorage.getItem('servo-test') || 'empty';
  const cookie = document.cookie.split('; ').find(value => value.startsWith('servo_test='))?.split('=')[1] || 'empty';
  if (storage !== 'read') {
    localStorage.setItem('servo-test', storage);
    document.cookie = 'servo_test=' + storage + '; SameSite=Lax; path=/; Max-Age=3600';
  }
  document.title = 'storage:' + before + ':' + cookie;
}
if (q.get('permission')) setTimeout(() => {
  Notification.requestPermission().then(answer => { document.title = 'permission:' + answer; });
}, 500);
if (q.get('pick')) setTimeout(() => { document.title = 'picking'; c.click(); }, 1000);
if (q.get('popup')) setTimeout(() => {
  const w = window.open('/?child=1', 'signin', 'width=500,height=600');
  document.title = 'opened:' + (w ? 'window' : 'null');
  setTimeout(() => { document.title = 'closed:' + (w && w.closed); }, 4000);
}, 1000);
if (q.get('child')) setTimeout(() => {
  const pending = window.open('about:blank');
  pending.document.write('<script>setTimeout(()=>fetch("/orphan-fired"),2000)<\\/script>');
  window.opener.document.title = 'opener-preserved';
  window.close();
}, 500);
if (q.get('tab')) setTimeout(() => { window.open('/?opened=1'); document.title = 'asked'; }, 1000);
if (q.get('opened')) document.title = 'second tab';
</script>"""


class Startup(ctypes.Structure):
    _fields_ = [('cb', wt.DWORD), ('reserved', wt.LPWSTR), ('desktop', wt.LPWSTR),
                ('title', wt.LPWSTR), ('x', wt.DWORD), ('y', wt.DWORD),
                ('width', wt.DWORD), ('height', wt.DWORD), ('chars_x', wt.DWORD),
                ('chars_y', wt.DWORD), ('fill', wt.DWORD), ('flags', wt.DWORD),
                ('show', wt.WORD), ('reserved_size', wt.WORD),
                ('reserved_bytes', ctypes.c_void_p), ('stdin', wt.HANDLE),
                ('stdout', wt.HANDLE), ('stderr', wt.HANDLE)]


class ProcessInfo(ctypes.Structure):
    _fields_ = [('process', wt.HANDLE), ('thread', wt.HANDLE),
                ('pid', wt.DWORD), ('tid', wt.DWORD)]


def launch_on_desktop(arguments, env, log):
    import msvcrt
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.CreateProcessW.argtypes = [wt.LPCWSTR, wt.LPWSTR, ctypes.c_void_p,
        ctypes.c_void_p, wt.BOOL, wt.DWORD, ctypes.c_void_p, wt.LPCWSTR,
        ctypes.POINTER(Startup), ctypes.POINTER(ProcessInfo)]
    kernel.CloseHandle.argtypes = [wt.HANDLE]
    startup = Startup()
    startup.cb = ctypes.sizeof(startup)
    startup.desktop = 'WinSta0\\' + DESKTOP_NAME
    def object_name(handle):
        value = ctypes.create_unicode_buffer(512)
        needed = wt.DWORD()
        if not user32.GetUserObjectInformationW(handle, 2, value, ctypes.sizeof(value), ctypes.byref(needed)):
            raise ctypes.WinError(ctypes.get_last_error())
        return value.value
    log.write('Launcher: ' + json.dumps({
        'window_station': object_name(user32.GetProcessWindowStation()),
        'thread_desktop': object_name(user32.GetThreadDesktop(kernel.GetCurrentThreadId())),
        'created_desktop': object_name(DESKTOP),
        'requested_desktop': startup.desktop,
    }) + '\n')
    log.flush()
    startup.flags = 0x100  # STARTF_USESTDHANDLES
    handle = msvcrt.get_osfhandle(log.fileno())
    os.set_handle_inheritable(handle, True)
    startup.stdout = startup.stderr = handle
    environment = ctypes.create_unicode_buffer('\0'.join(f'{k}={v}' for k, v in sorted(env.items())) + '\0\0')
    command = ctypes.create_unicode_buffer(subprocess.list2cmdline(arguments))
    info = ProcessInfo()
    try:
        if not kernel.CreateProcessW(None, command, None, None, True, 0x08000400,
                                     environment, str(REPO), ctypes.byref(startup), ctypes.byref(info)):
            raise ctypes.WinError(ctypes.get_last_error())
    finally:
        os.set_handle_inheritable(handle, False)
    kernel.CloseHandle(info.thread)
    kernel.CloseHandle(info.process)
    return info.pid


class App:
    def __init__(self, executable, url, profile=None, engine='servo'):
        self.work = REPO / 'target' / 'native-servo' / str(uuid.uuid4())
        self.work.mkdir(parents=True)
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0))
            self.port = s.getsockname()[1]
        env = {k: v for k, v in os.environ.items() if not k.startswith('FELARIS_')}
        self.profile = profile or self.work / 'profile'
        env.update(FELARIS_PROFILE_DIR=str(self.profile),
                   FELARIS_SMOKE_TEST=str(self.work / 'disabled.json'),
                   FELARIS_SERVO_TEST_PERMISSIONS='1',
                   FELARIS_SMOKE_BROWSER_ARGUMENTS=f'--remote-debugging-port={self.port} --remote-debugging-address=127.0.0.1')
        self.log_path = self.work / 'app.log'
        self.log = self.log_path.open('w', encoding='utf-8')
        self.pid = launch_on_desktop([str(executable), f'--engine={engine}', url], env, self.log)
        try:
            self.eval('true')
        except Exception:
            self.close()
            raise

    def eval(self, expression):
        result = subprocess.run(['node', str(REPO / 'tests/native_command.mjs'), str(self.port), expression],
                                timeout=30, capture_output=True, text=True)
        if result.returncode:
            self.log.flush()
            details = self.log_path.read_text(encoding='utf-8', errors='replace')[-4000:]
            raise RuntimeError(f'Felaris debugger command failed: {result.stderr.strip()}\nApp log: {details}')
        return result.stdout.strip()

    def titles(self):
        self.log.flush()
        return [line.split(', ', 2)[2].rstrip(')\n').strip('"')
                for line in self.log_path.read_text(encoding='utf-8', errors='replace').splitlines()
                if line.startswith('Title(')]

    def wait_title(self, wanted, timeout=20):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if any(wanted in title for title in self.titles()):
                return True
            time.sleep(0.2)
        return False

    def exit_cleanly(self):
        kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
        kernel.OpenProcess.restype = wt.HANDLE
        kernel.WaitForSingleObject.argtypes = [wt.HANDLE, wt.DWORD]
        kernel.CloseHandle.argtypes = [wt.HANDLE]
        handle = kernel.OpenProcess(0x100000, False, self.pid)
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            self.eval("Felaris.send('exit'), true")
            exited = kernel.WaitForSingleObject(handle, 10000) == 0
            self.exited = exited
            return exited
        finally:
            kernel.CloseHandle(handle)

    def close(self, remove_profile=True):
        if not getattr(self, 'exited', False):
            subprocess.run(['taskkill', '/PID', str(self.pid), '/T', '/F'], capture_output=True)
        self.log.close()
        if remove_profile:
            shutil.rmtree(self.profile, ignore_errors=True)


def check(name, condition, detail=''):
    RESULTS.append({'name': name, 'passed': bool(condition), 'detail': detail if not condition else ''})
    print(('PASS ' if condition else 'FAIL ') + name + (f' ({detail})' if detail and not condition else ''))
    return condition


def run(executable, only):
    server = http.server.ThreadingHTTPServer(('0.0.0.0', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f'http://127.0.0.1:{server.server_port}'
    downloads = downloads_folder()
    results = []

    def scenario(name):
        return not only or name in only

    try:
        for engine in ('webview2', 'servo'):
            if scenario(f'startup-{engine}'):
                app = App(executable, origin, engine=engine)
                try:
                    results.append(check(f'{engine} private-desktop startup responds', app.eval('true') == 'true'))
                    results.append(check(f'{engine} private-desktop startup exits cleanly', app.exit_cleanly()))
                finally:
                    app.close()
        if scenario('download'):
            target = downloads / f'{RUN_PREFIX}-naïve.txt'
            target.unlink(missing_ok=True)
            app = App(executable, f'{origin}/?go=attach')
            try:
                deadline = time.time() + 20
                while time.time() < deadline and not target.exists():
                    time.sleep(0.2)
                time.sleep(1)
                results.append(check('download saves under the server file name', target.exists()))
                results.append(check('download is complete', target.exists() and target.stat().st_size == 8000))
                mark = Path(str(target) + ':Zone.Identifier')
                results.append(check('download has the Mark of the Web',
                                     target.exists() and 'ZoneId=3' in mark.read_text()))
                titles = app.titles()
                results.append(check('page stays after download', titles[-1:] == ['leaving'], str(titles[-3:])))
            finally:
                app.close()
                target.unlink(missing_ok=True)

        if scenario('cancel'):
            target = downloads / f'{RUN_PREFIX}-slow.bin'
            target.unlink(missing_ok=True)
            app = App(executable, f'{origin}/?go=slow')
            try:
                deadline = time.time() + 20
                while time.time() < deadline and not target.exists():
                    time.sleep(0.2)
                time.sleep(1)
                results.append(check('slow download starts', target.exists()))
                app.eval("Felaris.send('close_tab', {id: 2}), true")
                time.sleep(1.5)
                text = app.eval('document.body.innerText')
                results.append(check('tab with a running download stays open', 'active download' in text, text[-200:]))
                app.eval("Felaris.send('open_downloads'), true")
                results.append(check('Open Downloads offers to cancel',
                                     answer_dialog(app.pid, 6)))  # IDYES
                deadline = time.time() + 10
                while time.time() < deadline and target.exists():
                    time.sleep(0.2)
                results.append(check('cancelled download removes its file', not target.exists()))
                time.sleep(1)
                text = app.eval('document.body.innerText')
                results.append(check('cancelled download reports interrupted', 'Download interrupted' in text, text[-200:]))
            finally:
                app.close()
                target.unlink(missing_ok=True)

        if scenario('stalled'):
            target = downloads / f'{RUN_PREFIX}-stall.bin'
            target.unlink(missing_ok=True)
            app = App(executable, f'{origin}/?go=stall')
            try:
                deadline = time.time() + 20
                while time.time() < deadline and not target.exists():
                    time.sleep(0.2)
                time.sleep(1)
                results.append(check('stalled download starts', target.exists()))
                app.eval("Felaris.send('close_tab', {id: 2}), true")
                time.sleep(1.5)
                text = app.eval('document.body.innerText')
                results.append(check('tab with a running download stays open', 'active download' in text, text[-200:]))
                app.eval("Felaris.send('open_downloads'), true")
                results.append(check('stalled download can be cancelled',
                                     answer_dialog(app.pid, 6)))  # IDYES
                deadline = time.time() + 10
                while time.time() < deadline and target.exists():
                    time.sleep(0.2)
                results.append(check('cancelled download removes its file', not target.exists()))
                time.sleep(1)
                text = app.eval('document.body.innerText')
                results.append(check('cancelled download reports interrupted', 'Download interrupted' in text, text[-200:]))
            finally:
                app.close()
                target.unlink(missing_ok=True)

        if scenario('memory'):
            target = downloads / f'{RUN_PREFIX}-large.bin'
            app = App(executable, f'{origin}/')
            try:
                if not app.wait_title('start'):
                    raise RuntimeError('Memory fixture did not load')
                time.sleep(1)
                baseline = peak_memory(app.pid)
                app.eval(f"Felaris.send('navigate', {{url: '{origin}/large'}}), true")
                deadline = time.time() + 30
                while time.time() < deadline:
                    if 'Download complete' in app.eval('document.body.innerText'):
                        break
                    time.sleep(0.2)
                growth = peak_memory(app.pid) - baseline
                results.append(check('large download completes',
                                     target.exists() and target.stat().st_size == 64 * 1024 * 1024))
                results.append(check('large download does not retain its whole body',
                                     growth < 32 * 1024 * 1024, f'peak grew {growth / 1024 / 1024:.1f} MiB'))
            finally:
                app.close()
                target.unlink(missing_ok=True)

        if scenario('color'):
            app = App(executable, f'{origin}/?pick=1')
            try:
                answer_dialog(app.pid, 1, {706: '18', 707: '52', 708: '86'})
                results.append(check('color picker fires input then change',
                                     app.wait_title('input:#123456 change:#123456'), str(app.titles()[-3:])))
            finally:
                app.close()

        if scenario('cache'):
            Handler.hits.clear()
            app = App(executable, f'{origin}/cache')
            try:
                app.wait_title('cache')
                time.sleep(2)
                same = lambda: Handler.hits.get(('127.0.0.1', '/same.js'), 0)
                other = lambda: Handler.hits.get(('127.0.0.2', '/other.js'), 0)
                first = (same(), other())
                app.eval("Felaris.send('reload'), true")
                time.sleep(3)
                results.append(check('normal reload uses the cache', (same(), other()) == first, f'{first} -> {(same(), other())}'))
                app.eval("Felaris.send('reload_fresh'), true")
                time.sleep(3)
                results.append(check('fresh reload refetches only the page host',
                                     (same(), other()) == (first[0] + 1, first[1]), f'{first} -> {(same(), other())}'))
            finally:
                app.close()

        if scenario('privacy'):
            app = App(executable, f'{origin}/?storage=public')
            profile = app.profile
            try:
                results.append(check('regular storage starts empty', app.wait_title('storage:empty:empty')))
                app.eval("Felaris.send('new_private_tab'), true")
                time.sleep(1)
                before_private = len(app.titles())
                app.eval(f"Felaris.send('navigate', {{url:'{origin}/?storage=private'}}), true")
                deadline = time.time() + 20
                while time.time() < deadline and not any('storage:empty:empty' in t for t in app.titles()[before_private:]):
                    time.sleep(0.2)
                time.sleep(1)
                results.append(check('private storage cannot read regular cookies or localStorage',
                                     app.titles()[-1:] == ['storage:empty:empty'], str(app.titles()[-3:])))
                app.eval("Felaris.send('switch_tab', {id:2}), true")
                app.eval(f"Felaris.send('navigate', {{url:'{origin}/?storage=read'}}), true")
                results.append(check('private writes do not alter regular storage', app.wait_title('storage:public:public')))
                results.append(check('browser exits through its normal command', app.exit_cleanly()))
            finally:
                app.close(remove_profile=False)
            restored = App(executable, f'{origin}/?storage=read', profile=profile)
            try:
                results.append(check('regular storage persists after restart', restored.wait_title('storage:public:public')))
                restored.eval("Felaris.send('new_private_tab'), true")
                time.sleep(1)
                restored.eval(f"Felaris.send('navigate', {{url:'{origin}/?storage=read'}}), true")
                results.append(check('private storage is empty after restart', restored.wait_title('storage:empty:empty')))
            finally:
                restored.close()

        if scenario('private-session'):
            app = App(executable, f'{origin}/')
            try:
                app.wait_title('start')
                app.eval("Felaris.send('new_private_tab'), true")
                time.sleep(1)
                app.eval(f"Felaris.send('navigate', {{url:'{origin}/?storage=private'}}), true")
                results.append(check('private tab writes storage', app.wait_title('storage:empty:empty')))
                app.eval("Felaris.send('switch_tab', {id:2}), true")
                app.eval("Felaris.send('close_tab', {id:3}), true")
                time.sleep(1)
                before = len(app.titles())
                app.eval("Felaris.send('new_private_tab'), true")
                time.sleep(1)
                app.eval(f"Felaris.send('navigate', {{url:'{origin}/?storage=read'}}), true")
                deadline = time.time() + 20
                while time.time() < deadline and not any(t.startswith('storage:') for t in app.titles()[before:]):
                    time.sleep(0.2)
                later = [t for t in app.titles()[before:] if t.startswith('storage:')]
                results.append(check('closing every private tab ends the private session',
                                     later[-1:] == ['storage:empty:empty'], str(later)))
            finally:
                app.close()

        if scenario('crash'):
            app = App(executable, f'{origin}/')
            try:
                if not app.wait_title('start'):
                    raise RuntimeError('Crash fixture did not load')
                query = f"Get-CimInstance Win32_Process -Filter 'ParentProcessId={app.pid}' | Where-Object {{ $_.CommandLine -match '--content-process' }} | ForEach-Object {{ $_.ProcessId }}"
                children = subprocess.check_output(['powershell', '-NoProfile', '-Command', query], text=True).split()
                results.append(check('page content uses child processes', len(children) > 0))
                killed = 0
                for child in children:
                    result = subprocess.run(['taskkill', '/PID', child, '/F'], capture_output=True)
                    if result.returncode == 0:
                        killed += 1
                    else:
                        remaining = subprocess.check_output(
                            ['tasklist', '/FI', f'PID eq {child}', '/FO', 'CSV', '/NH'], text=True)
                        if f'"{child}"' in remaining:
                            raise RuntimeError(f'Could not terminate owned content process {child}: {result.stderr!r}')
                results.append(check('at least one live content process was terminated', killed > 0))
                app.wait_title('Page unavailable', timeout=15)
                sidebar = app.eval('document.body.innerText')
                results.append(check('browser survives a content-process crash',
                                     'Page unavailable' in app.titles() and 'New tab' in sidebar, sidebar[-600:]))
                before = len(app.titles())
                app.eval("Felaris.send('reload'), true")
                deadline = time.time() + 20
                while time.time() < deadline and 'start' not in app.titles()[before:]:
                    time.sleep(0.2)
                results.append(check('reload recovers crashed page', 'start' in app.titles()[before:],
                                     str(app.titles()[before:]) + app.eval('document.body.innerText')[-600:]))
            finally:
                app.close()

        if scenario('permission'):
            for button, expected in [('allow', 'granted'), ('block', 'denied')]:
                app = App(executable, f'{origin}/?permission=1')
                try:
                    time.sleep(2)
                    subprocess.run(['node', str(REPO / 'tests/native_command.mjs'), str(app.port),
                        f"document.getElementById('{button}').click(), true", '', '',
                        "document.getElementById('allow')?.disabled === false"],
                        check=True, timeout=30, capture_output=True)
                    results.append(check(f'permission {button} reaches Servo',
                                         app.wait_title('permission:' + expected), str(app.titles()[-3:])))
                finally:
                    app.close()

        if scenario('popup'):
            app = App(executable, f'{origin}/?popup=1')
            try:
                results.append(check('sign-in popup preserves opener', app.wait_title('opener-preserved', 20),
                                     str(app.titles()[-3:])))
                results.append(check('window.close reports a closed popup', app.wait_title('closed:true')))
                results.append(check('closing popup releases its pending children',
                                     not any(path == '/orphan-fired' and count for (_, path), count in Handler.hits.items())))
            finally:
                app.close()
    finally:
        server.shutdown()
    print(f'{sum(results)}/{len(results)} checks passed')
    return all(results)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default=str(REPO / 'target/debug/felaris.exe'))
    parser.add_argument('--only', nargs='*', default=[],
                        choices=['startup-webview2', 'startup-servo', 'download', 'cancel', 'stalled', 'memory', 'color', 'cache', 'popup', 'permission', 'privacy', 'private-session', 'crash'])
    arguments = parser.parse_args()
    DESKTOP_NAME = 'FelarisTests-' + str(uuid.uuid4())
    DESKTOP = user32.CreateDesktopW(DESKTOP_NAME, None, None, 0, 0x1FF, None)
    if not DESKTOP:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        passed = run(Path(arguments.exe).resolve(), arguments.only)
    finally:
        user32.CloseDesktop(DESKTOP)
        report = REPO / 'target/native-servo/results.json'
        report.parent.mkdir(parents=True, exist_ok=True)
        report.write_text(json.dumps(RESULTS, indent=2), encoding='utf-8')
    raise SystemExit(0 if passed else 1)
