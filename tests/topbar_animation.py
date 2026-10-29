"""Keep the visible top bar stable while the floating sidebar animates."""
import argparse
from pathlib import Path
import subprocess

from native_app import NativeApp


def check_topbar_animation(executable):
    app = NativeApp(executable, 'topbar-animation')
    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([], log)
            subprocess.run(['node', str(Path(__file__).with_name('topbar_animation.mjs')),
                            str(app.port), str(app.artifacts)], check=True, timeout=30)
    finally:
        app.close()
        print(f'Top bar animation evidence: {app.artifacts}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/veluno.exe')
    check_topbar_animation(parser.parse_args().exe)
