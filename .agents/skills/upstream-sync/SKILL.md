---
name: upstream-sync
description: Fetch updates from upstream remote or git repository, compare against local work, and run in-memory pre-merge conflict simulation before applying. Use when syncing upstream, checking for merge conflicts, or inspecting incoming remote changes.
---

# Upstream Sync & Conflict Inspector

Inspect, compare, and simulate incoming upstream changes against the current branch without modifying the working tree or dirtying Git index.

## Prerequisites

- Local git repository with at least one commit.
- An upstream source: either a configured remote name (e.g. `upstream`) or a direct git URL (`https://...`).

## Step 1: Pre-flight Status Check

1. Run `git status` to observe any uncommitted local modifications.
2. Confirm current branch name with `git rev-parse --abbrev-ref HEAD`.
3. Check configured remotes with `git remote -v`.

## Step 2: Run the Conflict Detection Script

Execute the bundled synchronization script relative to the project root:

```bash
# Using Python directly (recommended on Windows):
python scripts/upstream_sync.py --upstream <upstream_name_or_url> --branch <branch_name>

# Or with bash wrapper:
./scripts/upstream-sync.sh --upstream <upstream_name_or_url> --branch <branch_name>

# Or with PowerShell:
.\scripts\upstream-sync.ps1 -Upstream <upstream_name_or_url> -Branch <branch_name>
```

Optional flags:
- `--show-commits`: List incoming commit summaries.
- `--allow-unrelated-histories`: Allow checking branches that do not share a common ancestor commit.

## Step 3: Interpret Results

The script runs `git merge-tree --write-tree HEAD FETCH_HEAD` in memory:

1. **Clean Merge (`RESULT: CLEAN MERGE`)**:
   - Zero conflicts detected.
   - Shows the list of files modified upstream.
   - Safe to proceed with `git merge FETCH_HEAD` or `git rebase FETCH_HEAD`.

2. **Conflicts Detected (`RESULT: CONFLICTS DETECTED`)**:
   - Lists exact conflicting files and conflict types (content conflict, modify/delete).
   - Shows where our local changes overlap with upstream changes.
   - **Do not blindly merge.** Follow the resolution workflow:
     1. Create a safety backup branch: `git branch backup-pre-upstream-sync`
     2. Create a test merge branch: `git checkout -b test-upstream-sync`
     3. Perform test merge: `git merge FETCH_HEAD`
     4. Resolve conflicted files individually.
     5. Test and verify build.
     6. Fast-forward main branch once clean.

## Completion Criteria

- Upstream commits fetched into `FETCH_HEAD`.
- Commit delta (ahead/behind) reported.
- Pre-merge conflict simulation evaluated and reported.
- Decision made whether to apply changes or resolve conflicts.
