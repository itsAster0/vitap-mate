---
name: test-on-emulator
description: Run the app on the Android emulator and check a change there, with logs, the UI tree, the state dump and the demo account. Use after a change to the app or the Rust core, before saying it works.
---

# Test on the emulator

Test on the emulator, not on the user's own phone: the phone holds their real
cache and accounts. Never clear its data, uninstall or sign out on it.

`ANDROID_HOME` picks the SDK (default `~/Library/Android/sdk`) and
`ANDROID_SERIAL` the device for the adb tasks.

## Build and run

```sh
mise run rust-android-dev       # after a Rust change (arm64 only, quick); rust-android for 32-bit too
./kotlin run -d emulator-5554   # builds and starts the debug app
```

The Kotlin toolchain doesn't notice a new `.so` on its own; the rust-android
tasks drop the cached Android build for it.

## Sign in

Without a VTOP account use the demo account: `DEMO2026`, password
`vitapmate-demo` (`rust/vtop-ffi/src/demo.rs`). It asks VTOP nothing and reads
every page from the backend's `/guides/demo.json`. Outing writes, downloads, the
web view and forwarding answer as unavailable there, so test those with a real
account, and only with the user's OK.

## Look at it

```sh
mise run logs       # follow logcat, tag vitapmate (Kotlin and Rust)
mise run logs-pull  # the app's own log, cache/logs/app.jsonl
mise run ui-dump    # the on-screen UI tree; test tags show as resource-id (tab.attendance, screen.timetable)
mise run state      # route, settings, pages and the last 20 flows, as JSON
mise run trace      # 10 s of Perfetto into traces/
```

Every log line carries its flow id and `regno`; follow one flow by its id.

## Broadcasts (debuggable builds only)

```sh
adb shell am broadcast -p com.vitap_pal.app -a vitapmate.DUMP          # what `mise run state` reads
adb shell am broadcast -p com.vitap_pal.app -a vitapmate.SYNC_NOW      # a background sync now
adb shell am broadcast -p com.vitap_pal.app -a vitapmate.SYNC_NOW --ez simulate true  # roll pages back first
```

`simulate` rolls the saved pages back, so only send it to the emulator.

## What to check

- The screen with no cache, after a refresh, and offline.
- The route you changed, and Direct (Settings → Connection).
- No cookies, passwords, OTPs, CSRF tokens or keys in the logs.
- Say what you ran and what you saw; a build alone isn't a test.
