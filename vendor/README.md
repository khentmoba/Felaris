# Patched Servo crates

These are Servo 0.7.0 crates from crates.io with Felaris patches applied. `Cargo.toml` points at
them through `[patch.crates-io]`. Only the `servo` feature builds them; the default WebView2 build
does not.

They stay under Servo's license, MPL-2.0 (see the header in each file). Servo's AI usage policy
does not accept these patches upstream, so they live here.

| Patch | Crates | Change |
| --- | --- | --- |
| `patches/private-browsing.patch` | `servo`, `servo-embedder-traits`, `servo-constellation`, `servo-storage` | Adds `WebViewBuilder::private`. Top-level webviews can use Servo's private network state, and private storage no longer shares the profile directory with public storage. A crashed private page also stays private. |
| `patches/decode-credentials.patch` | `servo-net` | Percent-decodes URL and HTTP-authentication credentials before building the `Authorization` header, so passwords with spaces, `@`, `:` and similar characters work. |
| `patches/color-input-events.patch` | `servo-script` | Fires `input` and then `change` when a color picker answer changes an `<input type=color>`'s value. |
| `patches/site-cache-clearing.patch` | `servo`, `servo-net-traits`, `servo-net` | Adds `NetworkManager::clear_cache_for_sites`, which drops the memory and disk HTTP cache entries for some hosts and their subdomains. |
| `patches/downloads.patch` | `servo`, `servo-net` | A top-level navigation whose response is an attachment, or has a type Servo cannot display, becomes a download: the page gets `204 No Content` (so it stays), `WebViewDelegate::request_download` picks a file, and the same response body is written there. `notify_download_finished` reports the result. |

| `patches/production-hardening.patch` | `servo`, `servo-embedder-traits`, `servo-script`, `servo-net` | Adds requesting-origin metadata for permissions and hardens downloads with bounded HTTP streaming, asynchronous cancellation and retained file handles. Apply this after the five patches above. |

## Upgrading Servo

1. Delete the crate directories here and copy the new versions from
   `~/.cargo/registry/src/index.crates.io-*/` (drop the version suffix, and the
   `.cargo_vcs_info.json` and `.cargo-ok` files).
2. From the repository root, apply `private-browsing`, `decode-credentials`, `color-input-events`,
   `site-cache-clearing`, and `downloads`, in that order, then `production-hardening`:
   `git apply --directory=vendor vendor/patches/<name>.patch`.
   Fix any rejected hunks by hand and regenerate the patches.
3. Bump `servo` in `Cargo.toml`, then build with `scripts/servo.ps1` and run the checks in
   `docs/servo.md`.

If a new Servo release includes an equivalent fix, drop that patch, and drop any crate that no
longer has one.
