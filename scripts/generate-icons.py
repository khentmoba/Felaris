"""Generate Windows icons from veluno-icon.png (pip install Pillow==12.2.0)."""
import argparse
import io
from pathlib import Path

from PIL import Image


ASSETS = Path(__file__).resolve().parent.parent / 'assets'
SIZES = (16, 24, 32, 48, 64, 128, 256)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Fail if generated icons are stale')
    args = parser.parse_args()
    with Image.open(ASSETS / 'veluno-icon.png') as source:
        source = source.convert('RGBA')
    buffer = io.BytesIO()
    source.save(buffer, format='ICO', sizes=[(size, size) for size in SIZES])
    outputs = {
        'icon.ico': buffer.getvalue(),
        'icon_64.rgba': source.resize((64, 64), Image.Resampling.LANCZOS).tobytes(),
    }
    stale = []
    for name, data in outputs.items():
        path = ASSETS / name
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                stale.append(name)
        else:
            path.write_bytes(data)
    if stale:
        parser.exit(1, f'Stale Windows branding: {", ".join(stale)}. Run python scripts/generate-icons.py\n')
    print('Windows icons match veluno-icon.png.' if args.check else 'Generated Windows icons.')


if __name__ == '__main__':
    main()
