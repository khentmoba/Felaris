#!/usr/bin/env python3
"""
Felaris Automated Build & Release Pipeline
Compiles Felaris in release mode and automatically publishes to Discord.
"""

import sys
import subprocess
import argparse
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description="Build Felaris and automatically publish to Discord.")
    parser.add_argument("--skip-discord", action="store_true", help="Build without publishing to Discord")
    parser.add_argument("--summary", "-s", help="Release changelog or summary")
    parser.add_argument("--dry-run", action="store_true", help="Run build and print payload without sending")
    args = parser.parse_args()

    print("==================================================")
    print(" Building Felaris (Release Mode)")
    print("==================================================")

    build_cmd = ["cargo", "build", "--release"]
    print(f"[Build] Executing: {' '.join(build_cmd)}")
    res = subprocess.run(build_cmd, cwd=PROJECT_ROOT)
    if res.returncode != 0:
        print("[Build] Compilation failed.")
        sys.exit(res.returncode)

    exe_path = PROJECT_ROOT / "target" / "release" / "felaris.exe"
    if not exe_path.is_file():
        print(f"[Build] Binary not found at {exe_path}")
        sys.exit(1)

    size_mb = exe_path.stat().st_size / (1024 * 1024)
    print(f"[Build] Compilation successful: {exe_path.name} ({size_mb:.2f} MB)")

    if args.skip_discord:
        print("[Discord] Skipping Discord notification (--skip-discord requested).")
        return

    print("\n==================================================")
    print(" Notifying Discord #felaris-releases")
    print("==================================================")

    notify_script = PROJECT_ROOT / "scripts" / "notify_discord.py"
    notify_cmd = [sys.executable, str(notify_script)]
    if args.summary:
        notify_cmd.extend(["--summary", args.summary])
    if args.dry_run:
        notify_cmd.append("--dry-run")

    res = subprocess.run(notify_cmd, cwd=PROJECT_ROOT)
    if res.returncode == 0:
        print("[Discord] Release announcement and binary published successfully!")
    else:
        print("[Discord] Release notification encountered an error.")
        sys.exit(res.returncode)


if __name__ == "__main__":
    main()
