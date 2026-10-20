# WebView2 memory tuning

Measured with `python -B tests/native_memory.py` on Windows 11, WebView2 155.0.4283.62, release builds.
Each tab is its own `*.localhost` site (its own renderer) with a synthetic page: 200k-object JS heap,
3,000 paragraphs, and a 1200×800 canvas. MiB = summed private working set (Task Manager's "Memory")
/ private commit, across `felaris.exe` and every `msedgewebview2.exe` child. Headless test window.

## Hidden tabs get `MemoryUsageTargetLevel = Low` (30 tabs, 12 live WebViews, two runs each)

| Sample | Before | After |
|---|---|---|
| 1 tab | 245–247 / 441–442 | 246–247 / 441–445 |
| 10 tabs | 715–718 / 1071–1079 | 324–325 / 1068–1079 |
| 30 tabs | 800–801 / 1164–1165 | 380–382 / 1164–1176 |
| 30 tabs, idle 120 s | 641–644 / 989–998 | 586–588 / 995–1002 |
| after switching 10 tabs | 730–788 / 1101–1191 | 393–410 / 1130–1150 |

Commit does not change: `Low` trims resident pages, it does not free them. V8's idle GC faults part of
the trimmed memory back in while idle (tab renderers 74 → 301 MiB over 120 s).

| Timing (median, ms) | Before | After |
|---|---|---|
| New cross-site tab load | 143–149 | 144–145 |
| Switch to a hidden live tab | 10–11 | 10–11 |
| Wake a sleeping tab | 140–144 | 141–142 |
| Show → second animation frame | 3.1–3.3 (max 22–24) | 3.2–19.8 (max 22–23) |

Paint timing is bimodal: the next frame, or one frame later. The median landed one frame later in 2 of 4
short runs after the change and 0 of 4 before; baseline also landed there (20.2 ms) after a 33-minute idle.
The worst case is unchanged, so the cost is at most one 16 ms frame when switching back to a hidden tab.

## Hidden tabs sleep after 30 minutes (10 tabs, under the 12-WebView budget)

| Sample | Before | After |
|---|---|---|
| 10 tabs, idle 1,770 s | 579 / 928 (16 renderers) | 531 / 923 (16 renderers) |
| 10 tabs, idle 1,800 s | 579 / 929 (16 renderers) | 278 / 511 (6 renderers) |
| 10 tabs, idle 1,980 s | 579 / 928 (16 renderers) | 270 / 502 (6 renderers) |

Before this change, sessions under 12 tabs never slept.

## Measured and not shipped

- `--renderer-process-limit=1` for the UI environment: no change (807 vs 802 MiB), because site isolation
  still gives each `data:` UI view its own renderer.
- The UI views run in a second WebView2 browser tree (`Chrome` data directory). Its browser, GPU,
  network, storage, and crashpad processes cost about 84 MiB of private working set. Merging it into the
  tab environment would also merge extension settings, debug ports, and crash isolation, so it is left
  for a separate change.
