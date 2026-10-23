# Servo production readiness

Servo is an opt-in engine. WebView2 remains the production default. This draft is a checkpoint
for continuing the work, not a release candidate.

## Verified locally

- Separate content processes; an unexpected process exit produces an error page and Reload
  recovers the page while browser controls remain usable.
- HTTP downloads with bounded buffering, stalled cancellation, retained file ownership, Windows
  marking, and tab-close protection.
- Popup opener communication, script-initiated close, and recursive pending-child cleanup.
- Host-specific cache eviction and native color-picker input/change events.
- Permission Allow/Block decisions through Felaris's prompt. Permission-dependent backends remain
  disabled in normal browsing; iframe and worker requests fail closed.
- Regular cookies and localStorage persist across normal Exit and restart. Private cookies and
  localStorage are isolated and empty after restart. Closing the last private tab clears private
  cookies, web storage and cached responses in the running browser. Forced termination can lose
  unflushed regular cookies.

The combined tree passes 47 default and 49 Servo-feature Rust tests, 36 isolated-desktop Servo
runtime checks, JavaScript checks, 18 release guards, and the default WebView2 native smoke suite.
These results are local evidence. Hosted CI, packaging, and installer results must be assessed
for the PR's current head.

## Remaining production gates

- [ ] A Windows content sandbox with an enforced security boundary. Multiprocess alone is not one.
- [ ] A supported-site and workflow matrix, including sign-in and sustained browsing.
- [ ] Audio/video playback and the required device/media permission backends.
- [ ] Windows accessibility and real IME input validation.
- [ ] Composed window/dialog screenshots and visual review. The current headless suite proves
      behavior, not appearance or compositor correctness; no Servo screenshot baseline is retained.
- [ ] Storage coverage beyond cookies/localStorage (IndexedDB, private auth cache and HSTS state),
      plus profile-clearing semantics.
- [ ] Clean-machine Servo build, dependency/license packaging, installer and update verification.
- [ ] Passing hosted checks for the current PR head and review of remaining findings.

## Reproduction commands

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
node tests/browser.test.cjs
python -B tests/release.test.py
powershell -NoProfile -File scripts/servo.ps1 -RetryLockedCompiles
powershell -NoProfile -File scripts/servo.ps1 -Check -RetryLockedCompiles
python -u -I tests/native_servo.py
python -I tests/native_smoke.py --exe target/debug/felaris.exe
```

Runtime logs and JSON reports live under `target/native-servo` and `target/native-smoke`.
See [servo.md](servo.md) for the build recipe and supported behavior, and
[../vendor/README.md](../vendor/README.md) for the ordered patch set.
