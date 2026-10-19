#!/usr/bin/env python3
"""
Felaris Automated Build & Release Pipeline
Compiles Felaris in release mode, publishes to GitHub Releases, and notifies Discord.
"""

import sys
import json
import shutil
import subprocess
import argparse
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent


def get_cargo_version():
    cargo_path = PROJECT_ROOT / "Cargo.toml"
    if cargo_path.is_file():
        for line in cargo_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("version ="):
                return line.split("=")[1].strip(' "\'')
    return "0.1.0"


def sync_version_json(version, summary):
    manifest_path = PROJECT_ROOT / "version.json"
    data = {
        "version": version,
        "release_url": f"https://github.com/khentmoba/Felaris/releases/tag/v{version}",
        "download_url": f"https://github.com/khentmoba/Felaris/releases/download/v{version}/felaris.exe",
        "summary": summary,
    }
    manifest_path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"[Version] Synced version.json with v{version}")


def publish_github_release(version, summary, exe_path, dry_run=False):
    tag = f"v{version}"
    if not shutil.which("gh"):
        print("[GitHub] 'gh' CLI not found. Skipping GitHub Release creation.")
        return

    if dry_run:
        print(f"[DryRun] Would create GitHub release {tag} with asset: {exe_path}")
        return

    print(f"[GitHub] Creating GitHub Release {tag}...")
    cmd = [
        "gh",
        "release",
        "create",
        tag,
        str(exe_path),
        "--title",
        f"Felaris {tag}",
        "--notes",
        summary,
    ]
    res = subprocess.run(cmd, cwd=PROJECT_ROOT)
    if res.returncode == 0:
        print(f"[GitHub] Release {tag} successfully published to GitHub!")
    else:
        print(f"[GitHub] Warning: gh release create returned code {res.returncode}")


def main():
    parser = argparse.ArgumentParser(description="Build Felaris and automatically publish to Discord.")
    parser.add_argument("--skip-discord", action="store_true", help="Build without publishing to Discord")
    parser.add_argument("--skip-github", action="store_true", help="Build without creating a GitHub release")
    parser.add_argument("--summary", "-s", help="Release changelog or summary")
    parser.add_argument("--dry-run", action="store_true", help="Run build and print payload without sending")
    args = parser.parse_args()

    version = get_cargo_version()
    summary = args.summary or f"Felaris v{version} release — smooth, lightweight, Zen-inspired browser built in Rust."

    print("==================================================")
    print(f" Building Felaris v{version} (Release Mode)")
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

    sync_version_json(version, summary)

    if not args.skip_github:
        print("\n==================================================")
        print(" Publishing GitHub Release")
        print("==================================================")
        publish_github_release(version, summary, exe_path, dry_run=args.dry_run)

    if args.skip_discord:
        print("[Discord] Skipping Discord notification (--skip-discord requested).")
        return

    print("\n==================================================")
    print(" Notifying Discord #felaris-releases")
    print("==================================================")

    notify_script = PROJECT_ROOT / "scripts" / "notify_discord.py"
    notify_cmd = [sys.executable, str(notify_script)]
    if summary:
        notify_cmd.extend(["--summary", summary])
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
