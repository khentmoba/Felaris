# UI and motion refinement

The black theme, system fonts, browser actions, and native sidebar geometry remain in place. This pass makes New tab easier to find, gives the address field more room, and presents Home shortcuts as two columns of compact rows. Below 360px, Home uses one column. The Home keyboard hint now matches Ctrl+Shift+B.

## Before and after

These are synthetic fixtures rendered in the app's native WebView2 content view. The fixture uses the repository's sidebar, Home, palette, and shared browser script. Baseline HTML comes from commit `23a4f06`; after captures use this change. Animations were finished explicitly for still captures because the hidden fixture's animation clock can be throttled. These images demonstrate layout, not Windows composition or frame rate.

| Surface | Before | After |
| --- | --- | --- |
| Desktop, 1380×860 | [Overview](overview-1380-before.png) | [Overview](overview-1380-after.png) |
| Search, 1380×860 | [Search](search-1380-before.png) | [Search](search-1380-after.png) |
| Minimum window, 640×480 | [Overview](overview-640-before.png) | [Overview](overview-640-after.png) |
| Search, 640×480 | [Search](search-640-before.png) | [Search](search-640-after.png) |

## Motion proof

A native WebView2 probe started a `tab-layout` animation, changed a background tab's title, and checked the original animation's play state. With the baseline HTML, the animation was cancelled. With this change, it remained running. The same assertion is in `tests/ui.html`.

Tab geometry is measured only when the visible tab order or pin grouping changes. New rows hold their starting pose during the bounded stagger. The palette scrim now animates opacity on a separate noninteractive layer, and dismissal starts at its current opacity. Animated blur is removed from the panel. Existing tests exercise rapid reopening, single submission, keyboard focus, IME input, and reduced motion. No frame-rate improvement is claimed from screenshots.

## Validation

Passed locally:

- `cargo build --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --locked` — 48 tests
- `node tests/browser.test.cjs`
- `python -B tests/release.test.py` — 18 tests
- `python -B tests/native_smoke.py`
- `python -B tests/native_regressions.py` — includes 108 UI checks at 1380×860 with normal motion and 640×480 with reduced motion; native top bar width stayed at 640px over 381 reveal/hide/reversal samples
- `python -B tests/native_tools.test.py`
- Impeccable detector on all three changed UI surfaces — no findings

The screenshot fixture timeout was caused by waiting on animations in a hidden WebView; explicit animation completion produced the still captures. The native interaction tests passed without that workaround.

Installer packaging and lifecycle validation run in Windows CI. No release is published by this task.
