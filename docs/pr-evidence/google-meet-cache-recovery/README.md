# Google Meet startup recovery evidence

`before.png` is the native Veluno menu from the pre-fix debug executable. `after.png` is the same menu from the updated executable. Both captures use an isolated profile and a local synthetic page; no personal browsing data appears.

The native cache-recovery regression reproduces a service worker that serves a stale 503 JavaScript bootstrap. It confirms that the app reports the script failure, normal Reload clears that origin's CacheStorage and service worker, and the page starts. The test also confirms that cookies, local storage, IndexedDB, notification permission, and a cache for a different origin survive.
