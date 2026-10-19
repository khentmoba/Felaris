#!/usr/bin/env python3
"""
Felaris Upstream Sync & Conflict Inspector
Fetches updates from upstream and analyzes potential conflicts against local work
WITHOUT modifying the working tree or dirtying Git index.
"""

import sys
import subprocess
import argparse
import os

def run_git(args, check=True):
    result = subprocess.run(["git"] + args, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if check and result.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed:\n{result.stderr}")
    return result

def get_current_branch():
    res = run_git(["rev-parse", "--abbrev-ref", "HEAD"], check=False)
    if res.returncode == 0:
        branch = res.stdout.strip()
        if branch != "HEAD":
            return branch
    return "main"

def get_dirty_files():
    res = run_git(["status", "--porcelain"])
    lines = [line.strip() for line in res.stdout.splitlines() if line.strip()]
    return lines

def main():
    parser = argparse.ArgumentParser(description="Fetch upstream updates and inspect potential conflicts.")
    parser.add_argument("--upstream", "-u", default="upstream", help="Upstream remote name or git URL (default: 'upstream')")
    parser.add_argument("--branch", "-b", default=None, help="Upstream branch to inspect (default: current branch or 'main')")
    parser.add_argument("--show-commits", "-c", action="store_true", help="Print details of incoming upstream commits")
    parser.add_argument("--allow-unrelated-histories", action="store_true", help="Allow merging unrelated histories during check")
    args = parser.parse_args()

    branch = args.branch or get_current_branch()
    print(f"\n=======================================================")
    print(f" Felaris Upstream Sync & Conflict Detection")
    print(f" Target Upstream: {args.upstream} | Branch: {branch}")
    print(f"=======================================================\n")

    # 1. Check local working tree state
    dirty = get_dirty_files()
    if dirty:
        print(f"[*] Note: You have {len(dirty)} uncommitted local file(s):")
        for f in dirty[:5]:
            print(f"    - {f}")
        if len(dirty) > 5:
            print(f"    - ... and {len(dirty) - 5} more")
        print()

    # 2. Check if upstream remote exists, or if a URL was passed
    remotes = run_git(["remote"]).stdout.splitlines()
    upstream_target = args.upstream
    if args.upstream not in remotes and not (args.upstream.startswith("http://") or args.upstream.startswith("https://") or args.upstream.startswith("git@")):
        print(f"[!] Warning: Remote '{args.upstream}' is not configured.")
        print(f"    Existing remotes: {', '.join(remotes) if remotes else 'none'}")
        print(f"    You can set one up with:")
        print(f"      git remote add upstream <url>")
        print(f"    Or specify a full URL:")
        print(f"      python scripts/upstream_sync.py --upstream https://github.com/owner/repo.git\n")
        return 1

    # 3. Fetch from upstream
    print(f"[*] Fetching latest updates from '{upstream_target}' (branch: {branch})...")
    fetch_cmd = ["fetch", upstream_target, branch]
    fetch_res = run_git(fetch_cmd, check=False)
    if fetch_res.returncode != 0:
        print(f"[x] Failed to fetch from upstream:")
        print(fetch_res.stderr.strip())
        return 1
    print("[+] Upstream fetched successfully into FETCH_HEAD.\n")

    # 4. Compare commits
    try:
        behind_res = run_git(["rev-list", "--count", "HEAD..FETCH_HEAD"], check=False)
        ahead_res = run_git(["rev-list", "--count", "FETCH_HEAD..HEAD"], check=False)
        
        if behind_res.returncode == 0 and ahead_res.returncode == 0:
            behind_count = int(behind_res.stdout.strip())
            ahead_count = int(ahead_res.stdout.strip())
            print(f"[*] Commit Status:")
            print(f"    - Upstream commits waiting (behind): {behind_count}")
            print(f"    - Local commits ahead of upstream:   {ahead_count}")
            print()

            if behind_count == 0:
                print("[+] You are already completely up to date with upstream! No changes to sync.\n")
                return 0

            if args.show_commits or behind_count <= 10:
                print(f"[*] Incoming commits ({min(behind_count, 10)} of {behind_count}):")
                log_res = run_git(["log", "--oneline", f"-{min(behind_count, 10)}", "HEAD..FETCH_HEAD"])
                for line in log_res.stdout.splitlines():
                    print(f"    {line}")
                print()
    except Exception as e:
        print(f"[*] Could not compute commit counts (histories may be unrelated): {e}")

    # 5. In-Memory Conflict Analysis using git merge-tree
    print("[*] Running in-memory pre-merge conflict simulation...")
    merge_tree_cmd = ["merge-tree", "--write-tree"]
    if args.allow_unrelated_histories:
        merge_tree_cmd.append("--allow-unrelated-histories")
    merge_tree_cmd.extend(["HEAD", "FETCH_HEAD"])

    mt_res = run_git(merge_tree_cmd, check=False)

    if mt_res.returncode == 0:
        print("\n=======================================================")
        print(" RESULT: CLEAN MERGE (NO CONFLICTS)")
        print("=======================================================")
        print("[+] Upstream changes can be cleanly integrated with no file conflicts.")
        
        # List modified files
        diff_files = run_git(["diff", "--name-status", "HEAD...FETCH_HEAD"], check=False)
        if diff_files.returncode == 0 and diff_files.stdout.strip():
            print("\n[*] Files modified upstream:")
            for line in diff_files.stdout.splitlines()[:15]:
                print(f"    {line}")
            total_lines = len(diff_files.stdout.splitlines())
            if total_lines > 15:
                print(f"    ... and {total_lines - 15} more files")
        
        print("\n[*] Recommended next step to apply:")
        print("    git merge FETCH_HEAD")
        print("    (or: git rebase FETCH_HEAD)")
        print()
        return 0
    else:
        print("\n=======================================================")
        print(" RESULT: CONFLICTS DETECTED!")
        print("=======================================================")
        print("[!] Merging upstream would cause conflicts with current work.")
        
        output_lines = (mt_res.stdout + "\n" + mt_res.stderr).splitlines()
        conflicts = [line for line in output_lines if "CONFLICT" in line]
        
        if conflicts:
            print(f"\n[*] Conflicting paths ({len(conflicts)} found):")
            for c in conflicts:
                print(f"    ! {c.strip()}")
        else:
            print("\n[*] Detailed conflict output:")
            for line in output_lines[:20]:
                print(f"    {line}")

        print("\n[*] Recommended resolution workflow:")
        print("    1. Create a safety branch: git branch backup-pre-upstream-sync")
        print("    2. Commit or stash any local uncommitted files")
        print("    3. Test merge on a temporary branch: git checkout -b test-upstream-sync && git merge FETCH_HEAD")
        print("    4. Resolve conflicting sections, then merge into your main branch.\n")
        return 2

if __name__ == "__main__":
    sys.exit(main())
