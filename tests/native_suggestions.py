"""Benchmark suggestions inside an isolated hidden WebView2 app."""
import argparse
from pathlib import Path
import subprocess
import sys

from native_app import NativeApp


def main():
    sys.stdout.reconfigure(encoding='utf-8')
    parser = argparse.ArgumentParser()
    parser.add_argument('--exe', default='target/debug/veluno.exe')
    parser.add_argument('--baseline', required=True)
    args = parser.parse_args()
    app = NativeApp(args.exe, 'native-suggestions')
    try:
        with (app.artifacts / 'app.log').open('w', encoding='utf-8') as log:
            app.start([], log)
            result = subprocess.run(
                ['node', str(Path(__file__).with_name('suggestions.bench.cjs')),
                 '--baseline', args.baseline, '--port', str(app.port)],
                capture_output=True, text=True, encoding='utf-8', timeout=60,
            )
            (app.artifacts / 'benchmark.log').write_text(
                result.stdout + result.stderr, encoding='utf-8',
            )
            print(result.stdout, end='')
            if result.returncode:
                raise RuntimeError(result.stderr)
    finally:
        app.close()
    print(f'Headless WebView2 benchmark artifacts: {app.artifacts}')


if __name__ == '__main__':
    main()
