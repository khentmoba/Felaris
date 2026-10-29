# Search and development build performance

Baseline: `66117f380af7e4872e35bdb4c579663cacba6f75`.
Measured on Windows/MSVC, Rust 1.93.0, Node 24.21.0, Python 3.12.10.

## Suggestion computation

The previous implementation scanned every history entry even after collecting
all eight empty-input results or seven exact-site matches. Once every result has
the maximum possible score, later entries cannot displace it: ties preserve the
original source order. The new implementation stops at that point. Other queries
still scan all sources so a later, stronger match can win. URL deduplication and
the explicit search/address fallback are unchanged.

`node tests/suggestions.bench.cjs --baseline 66117f3` checks 1,008 identical
before/after results, including mutated entries, replaced objects, reordered
sources, duplicate URLs with different titles, Unicode, and explicit addresses.
Timing uses the median of seven alternating samples with 2,000 history entries,
24 tabs, and 100 bookmarks, with 150 iterations per sample in Node.

| Workload | Before, ms | After, ms | Speedup |
| --- | ---: | ---: | ---: |
| Empty palette | 0.342 | 0.004 | 96.45x |
| Broad match | 0.448 | 0.437 | 1.02x |
| Exact site | 0.348 | 0.005 | 73.08x |
| Multiple words | 0.298 | 0.300 | 0.99x |
| No match | 0.238 | 0.248 | 0.96x |
| Replaced history objects | 0.376 | 0.037 | 10.15x |

Ratios use unrounded measurements. Replaced-object timings include cloning the
history. These are suggestion-computation timings, not whole-browser speedups.
Small differences in the full-scan workloads vary between runs.

`python -B tests/native_suggestions.py --baseline 66117f3` ran the same
comparison inside the final debug app's WebView2 sidebar and again returned
1,008 identical results. The native benchmark now uses 300 iterations per sample
instead of 20 to reduce timer quantization on the faster paths.

| WebView2 workload | Before, ms | After, ms | Speedup |
| --- | ---: | ---: | ---: |
| Empty palette | 0.161 | 0.002 | 80.50x |
| Broad match | 0.226 | 0.234 | 0.97x |
| Exact site | 0.187 | 0.002 | 80.00x |
| Multiple words | 0.178 | 0.179 | 0.99x |
| No match | 0.156 | 0.162 | 0.97x |
| Replaced history objects | 0.221 | 0.030 | 7.30x |

Native raw output is in
`target/native-suggestions/4ecdf3a4-f363-470f-98f6-2fe90442ad7c/benchmark.log`.
The 0.002 ms paths approach timer resolution, so the exact ratios are approximate.

## Cargo edit/build cycles

Three alternating rebuild samples touched `src/main.rs` without changing its
contents, then ran `cargo build --locked` with either the original full debug
information or `--config 'profile.dev.debug="line-tables-only"'`. Dependencies
were already built for both settings. Times include Cargo process startup.

| Round | Full debug, seconds | Line tables, seconds |
| --- | ---: | ---: |
| 1 | 1.878 | 2.186 |
| 2 | 2.017 | 1.652 |
| 3 | 1.985 | 1.618 |
| Median | 1.985 | 1.652 |

The median rebuild took 17% less time. This is a local warm-build measurement,
not a clean-build or startup guarantee. The development profile now uses line
tables, preserving backtrace source locations while omitting debugger locals and
types. `CARGO_PROFILE_DEV_DEBUG=2` restores full debug information. Release
settings are unchanged.

`cargo-blaze` 0.0.1 was inspected with `cargo info cargo-blaze` and its downloaded
README. It requires Rust 1.99.0 and supports macOS on Apple silicon or Linux on
x86-64/ARM64. It does not support this Windows/MSVC project. No global build tool
or toolchain was installed.

A separate release experiment with thin LTO and one codegen unit reduced the
executable from 2,821,120 to 2,707,456 bytes, about 4%, while increasing build time.
Those settings were not retained.

## Native permission fixture readiness

The final native rerun initially failed at `Timed out: location prompt` in
`tests/native_ui.mjs`. Its failure state is retained under
`target/native-regressions/90ac6e8b-a955-4220-9017-a5c078e5edd2/`.
The test waited only for `document.readyState === "complete"`. WebView2's
CDP target can advertise the destination before its JavaScript context leaves
`about:blank`, whose ready state is also complete. A location request in that
initial document disappears when navigation commits.

A focused probe reproduced two failures in six trials and captured
`{href:"about:blank",ready:"complete"}` at both failed requests. After requiring
the exact fixture URL in the document, all six requests ran at the destination
and showed the native location prompt. Probe logs are under
`target/location-probe/fb9b4812-63f5-49e5-8300-e1228f6af3bf/` before the fix and
`target/location-probe/2e49ffb2-30a4-4470-92ac-4612c233ce17/` after it.
The test now checks the exact URL as well as ready state, matching the adjacent
notification-permission fixture. No timeout or permission assertion was relaxed.

## Correctness and safety

The core invariant is that scanning stops only when every selected suggestion
has the maximum possible score. `tests/browser.test.cjs` verifies this with a
throwing URL getter after the final winning result. The same new test fails
against the baseline implementation and passes against the candidate. Additional
checks cover late high-score matches and duplicates that must not fill the list.
The existing mutation and eviction checks still pass. This verifies the invariant
by executing the actual suggestion function.

Local validation includes:

- `cargo fmt --check`.
- `cargo clippy --all-targets --locked -- -D warnings`.
- `cargo test --locked`, 49 tests passed.
- `node tests/browser.test.cjs`, browser JavaScript and sleep checks passed.
- `python -B tests/release.test.py`, 18 release-pipeline guard tests passed.
- `cargo build --locked`, plus a successful build with
  `--config profile.dev.debug=2` to verify the full-debug override.
- `python -B tests/native_smoke.py`.
- `python -B tests/native_regressions.py`, including native search selection,
  216 DOM checks across desktop/minimum sizes and reduced motion, page-state
  retention, downloads, and lazy restore/recovery with 100, 300, and 500 tabs.
- `python -B tests/native_tools.test.py`, including owned-process cleanup and
  deliberate animation/keyboard regressions rejected by the test tooling.

Installer lifecycle validation is left to Windows PR CI. Screenshots are not
applicable because the rendered interface and suggestion results are unchanged.
