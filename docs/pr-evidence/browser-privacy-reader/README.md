# Privacy, reader view, and bookmark import evidence

These screenshots come from `tests/native_ui.mjs` in the native regression run
for this change. The run uses a debug `felaris.exe`, a temporary profile, a
loopback fixture server, and a 640 x 480 Settings viewport. All page content is
synthetic.

- `settings-privacy.png` shows the top of Settings > Privacy and security.
  At this size, the tracking prevention and password controls sit below the fold.
- `settings-clear-data.png` shows the Clear browsing data panel. The "Cleared."
  status appears because the test ran a clear of cached files in the last hour.
- `reader-view.png` shows reader view over a local test article served from
  `127.0.0.1`.

No before screenshots exist. The clear browsing data panel and reader view are
new, and the Clear history row that the panel replaces was not captured.
