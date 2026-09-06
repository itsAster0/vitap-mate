# VTOP WebView validation

The app uses the native HTTP cache with normal revalidation. It does not cache
private pages in a separate application cache or override server cache headers.

## Automated checks

Run `flutter analyze --no-pub lib test` and `flutter test --no-pub` after restoring
locked dependencies. The WebView JavaScript test uses Node.js from PATH.

## Device checks

Use an authenticated test account on Android and iOS. No mobile device was
connected during implementation, so the checks below remain to be performed.

- Open Home five times under the same network conditions, then repeat with a
  shortcut. Confirm each reopening loads Home and each shortcut clicks once.
- Toggle compact and desktop modes, leave VTOP, reopen, and restart the app.
  Confirm settings apply on the first rendered page. Check light/dark mode,
  large text, narrow screens, and subsequent menu navigation.
- Expire the session. Confirm one automatic login attempt, OTP when required,
  and an explicit retry dialog after another redirect. Check Force Login.
- Switch accounts and sign out. Confirm retained content and VTOP cookies from
  the previous account are removed, including after closing the VTOP route.
- Disconnect while opening Home and while loading a menu. Confirm loading ends
  on failure and Open Home recovers without resubmitting forms. Check the
  slow-loading message under throttled networking.
- Download a document and verify authentication and filename handling.

## Performance evidence

The `vtop.webview` log category contains aggregate timings only:

- `prepareMs` includes settings, login/session preparation, and cookie sync.
- `cookieSyncMs` measures native cookie synchronization.
- `firstPageReadyMs` measures the first page load after the WebView is mounted.
- `pageLoadMs` measures each native navigation.
- `resourceTiming` reports page duration, static resource count, transferred
  bytes, and zero-transfer candidates. Zero transfer alone is not proof of a
  cache hit; browser timing restrictions and resource type also matter.

On a debug Android build, inspect through Chrome's remote WebView inspector.
Use Safari's Web Inspector for iOS. Keep "Disable cache" off. Inspect static
CSS, JavaScript, fonts and images for Cache-Control, ETag and Last-Modified,
cache hits, and 304 revalidation. Do not export cookies or authenticated bodies.

For cold measurements, clear website data only on the test device. For warm
measurements, reopen without clearing it. Compare medians of at least five runs
before and after the change on the same device and network. Separate session
preparation from page loading so server latency is not mistaken for UI cost.
Do not claim a speedup until these measurements exist.
