# Servo engine (experimental)

Felaris can render tab content with [Servo](https://servo.org) instead of WebView2. It is a
preview: WebView2 stays the default, and release builds do not include Servo.

## Build and run

```powershell
powershell -NoProfile -File scripts/servo.ps1 -Run
powershell -NoProfile -File scripts/servo.ps1 -Release -Run -- https://example.com
```

The script builds with `--features servo`, copies ANGLE's `libEGL.dll` and `libGLESv2.dll` next to
`felaris.exe`, and with `-Run` starts it with `--engine=servo`. Without `-Run`, launch the
executable yourself with that flag. If Servo cannot start, or the build lacks the feature, Felaris
says so and falls back to WebView2.

The first build compiles Servo and SpiderMonkey and takes a long time. It needs:

- The MSVC toolchain and Windows SDK, the same as a normal build.
- libclang for ANGLE's bindgen step. Set `LIBCLANG_PATH`, or the script downloads the
  `libclang` 18.1.1 wheel from PyPI into `target/servo-tools`. Python must be on `PATH` for that.

If C++ compiles fail at random with `C1056: cannot update the time date stamp field`, add
`-RetryLockedCompiles`. It routes C++ through `scripts/cl-retry.rs`, which retries those compiles.
Run clippy and tests with `-Check`; other cargo invocations change the environment and rerun
the C++ builds.

Use `FELARIS_PROFILE_DIR` to keep a Servo test run away from your normal profile and its
single-instance lock.

## How it fits

`src/engine/mod.rs` defines the `Engine` and `TabView` traits. `webview2.rs` is the default engine;
`servo.rs` is the preview. Browser chrome (sidebar, palette, topbar, prompts) always uses WebView2.
Only tab content goes through the engine.

- Page scripts run in separate content processes. Servo has no Windows content sandbox, so
  this is crash isolation, not a security boundary. `FELARIS_SERVO_SINGLE_PROCESS` is a
  diagnostic opt-out.
- All Servo tabs share one child window over the content area and one OpenGL (ANGLE) context.
  Only the active tab is painted.
- Servo wakes the app with `UserEvent::EngineWake`. The app then spins Servo and repaints when a
  new frame is ready.
- The surface window turns mouse, wheel and keyboard input into Servo input events. Felaris
  shortcuts are matched first, as in WebView2 tabs.
- Servo has no host IPC channel. `window.ipc.postMessage` is shimmed onto a tagged
  `console.log`, and the delegate passes it through the same token checks as WebView2
  (`app::content_message`).
- Internal pages (new tab, settings, history) load as `data:` URLs.
- `window.open` pages use the floating popup window and keep `window.opener`, including local
  sign-in callback pages. Servo does not report requested window features, so the embedder
  cannot distinguish a script's requested tab from a requested popup. Nested popups still
  reopen as normal tabs; only one floating popup is supported.
- Select menus, context menus, `alert`/`confirm`/`prompt`, file pickers, color pickers and HTTP
  sign-in use native Windows UI (`servo/controls.rs`). They open after Servo's event loop returns,
  because they run modal loops. Page dialogs and the sign-in dialog name the site in their title,
  so page text cannot pass for browser or Windows UI.
- IME text is inserted when composition ends; until then Windows shows it in the IME's own
  window, placed under the focused field.
- Private tabs use Servo's private network state and in-memory storage (`vendor/` patch).
  Clearing site data is refused in private tabs, because Servo clears public and private storage
  together.
- Clearing site data removes the storage Servo keeps for the site's registrable domain, which
  covers its subdomains. Clearing cached files and hard reloads drop the HTTP cache entries for
  the page's host and its subdomains.
- Downloads: a navigation to an attachment, or to a type Servo cannot display, saves the file to
  the Downloads folder without a prompt and leaves the page as it was. Names are made safe for
  Windows and numbered when taken, and files get the Mark of the Web so SmartScreen checks them.
  Open Downloads offers to cancel running transfers; otherwise it opens File Explorer.
  Transfers count toward Felaris's tab-close and sleep protection. HTTP bodies stream through
  a bounded queue rather than accumulating the whole file in memory. Cancellation wakes a
  stalled transfer. The writer retains the exclusively created file handle and removes failed
  output by handle on Windows.
- Supported top-level permission requests use Felaris's existing prompt and name the requesting
  document's origin. Frame and worker requests fail closed because the embedder does not yet
  track their document identities. Pending prompts are cancelled on navigation. Notifications,
  location and camera APIs remain disabled in normal browsing until their backends are supported.
  Debug diagnostic runs can enable notification permission requests using
  `FELARIS_SMOKE_TEST` and `FELARIS_SERVO_TEST_PERMISSIONS`; this tests decisions, not delivery.

Servo patches Felaris carries are in `vendor/`; see `vendor/README.md`.

## Not supported yet

- Printing and muting. These show a status message. Servo tabs play no audio or video.
- Download progress in bytes, pausing and resuming transfers.
- Downloads started by `<a download>` or from a script (`blob:` URLs) rather than by navigating.
- Notification delivery, location and camera backends. Routing a permission decision does not
  implement the feature that needs it.
- Multiple floating popups and keyboard shortcuts in the popup window.
- Browser extensions and the ad blocker.
- Showing IME composition text inside the page.

## Runtime checks

```powershell
powershell -NoProfile -File scripts/servo.ps1 -RetryLockedCompiles
powershell -NoProfile -File scripts/servo.ps1 -Check -RetryLockedCompiles
python -u -I tests/native_servo.py
```

The runtime suite creates a private Windows desktop that is never switched onto the user's
screen. It uses a new profile and unique download names, answers real native dialogs, and
retains event logs and `target/native-servo/results.json`. Tests cover download contents and
Windows marking, tab-close protection, cancellation while streaming and while stalled, bounded
memory growth, color events, cache isolation, local popup opener/close behavior, and permission
decisions. The separate Servo CI job runs this suite; it does not package or publish a release.

Passing these checks does not establish full web compatibility, a Windows sandbox, media
playback, accessibility support, or real IME behavior. WebView2 remains the production default.
