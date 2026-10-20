"""Own one isolated test app and its process tree, including capture runs."""
import os
from pathlib import Path
import shutil
import socket
import subprocess
import uuid


class NativeApp:
    def __init__(self, executable, suite):
        self.artifacts = (Path('target') / suite / str(uuid.uuid4())).resolve()
        self.artifacts.mkdir(parents=True)
        self.profile = self.artifacts / 'profile'
        self.executable = self.artifacts / 'felaris.exe'
        shutil.copy2(Path(executable).resolve(), self.executable)
        with socket.socket() as chrome, socket.socket() as content:
            chrome.bind(('127.0.0.1', 0))
            content.bind(('127.0.0.1', 0))
            self.port = chrome.getsockname()[1]
            self.content_port = content.getsockname()[1]
        self.environment = {key: value for key, value in os.environ.items() if not key.startswith('FELARIS_')}
        self.environment.update(
            FELARIS_PROFILE_DIR=str(self.profile),
            FELARIS_SMOKE_TEST=str(self.artifacts / 'disabled.json'),
            FELARIS_SMOKE_BROWSER_ARGUMENTS=f'--remote-debugging-port={self.port} --remote-debugging-address=127.0.0.1',
            FELARIS_SMOKE_CONTENT_BROWSER_ARGUMENTS=f'--remote-debugging-port={self.content_port} --remote-debugging-address=127.0.0.1',
        )
        self.process = None

    def start(self, arguments, log):
        if self.process is not None and self.process.poll() is None:
            raise RuntimeError('Stop the owned test app before starting another')
        self.process = subprocess.Popen([str(self.executable), *arguments], env=self.environment, stdout=log, stderr=log)
        return self.process

    def stop(self):
        if self.process is not None and self.process.poll() is None:
            result = subprocess.run(['taskkill', '/PID', str(self.process.pid), '/T', '/F'],
                                    capture_output=True, timeout=15)
            if result.returncode and self.process.poll() is None:
                raise RuntimeError(f'Could not stop owned app {self.process.pid}: {result.stderr!r}')
            self.process.wait(timeout=10)

    def close(self):
        self.stop()
        self.executable.unlink(missing_ok=True)
