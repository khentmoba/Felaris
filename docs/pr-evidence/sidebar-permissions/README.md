# Sidebar, search selection, and site permissions

## Screenshots

- `search-before.png` renders `src/new_tab.html` from base commit `320eb3e` in
  WebView2. The pointer is over X, but Facebook remains selected. Both rows have
  a background because CSS hover and combobox selection disagree.
- `search-after.png` renders the changed HTML with the same pointer position and
  synthetic Facebook/X shortcuts. Only X is selected. Both captures use a
  700 x 500 viewport and reduced motion. Favicon requests are blocked for stable
  images. No personal browsing data is included.
- `permission-after.png` captures the actual new permission WebView during a
  notification request from a local HTTP fixture. The origin is the isolated
  test server. A sanitized screenshot of the old native message box was not
  captured; the user supplied that baseline during the task.

## Runtime evidence

`python tests/native_regressions.py` runs the real Rust/WebView2 executable with
an isolated profile and dynamic loopback debugging ports. It checks:

- Three hover reveal/hide cycles keep the sidebar viewport at 260 CSS pixels.
  The original code changed it between 6 and 260 pixels.
- Windows child-window order keeps the collapsed sidebar above a newly created
  content view. The original code put the page above the sidebar's hover area.
- Native mouse movement onto X updates selection, and Enter opens that result.
- Notification prompts display the requesting origin and apply both Block and
  Allow through WebView2.
- A content page cannot answer a permission prompt through page IPC. After
  navigation cancels a pending location request, replaying its token cannot
  grant access. The test queries the browser's actual geolocation permission
  state, rather than checking only the prompt's DOM.
- All 60 DOM checks pass at 1380 x 860 and 640 x 480, including reduced motion.
  The existing keyboard, popup, download, sleeping-tab, and session-recovery
  checks also pass, including 100/300/500-tab sessions.

This proves the permission safety condition in the running app: only a response
for the current request from the trusted permission UI can grant access.
Camera/microphone hardware, live social-media sessions, and release installer
packaging were not exercised locally for this change.

## Other local checks

All passed:

```text
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked                         29 passed
node tests/browser.test.cjs
python -B tests/release.test.py             15 passed
git diff --check
```

The two new source comments explain WebView2 constraints: constant surface width
avoids resize flashes, and COM event arguments/deferrals stay on the UI thread.
Neither repeats the code, so the comment-cleanup pass kept both.
