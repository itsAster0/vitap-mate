<p align="center">
  <img src="assets/icon/icon.png" width="140" alt="vitapmate app icon">
</p>

# vitapmate

vitapmate is an unofficial Android app for VIT-AP students. It reads your timetable, attendance, marks and other pages from VTOP and saves them on the phone, so they open without a network and refresh in the background.

vitapmate is independent of VIT-AP University.

## Install

Install vitapmate from Google Play (`com.vitap_pal.app`), or download `vitapmate.apk` from the [nightly release](https://github.com/itsAster0/vitap-mate/releases/tag/nightly) and open it on the phone. A push to `main` rebuilds the nightly. It runs on 64-bit ARM phones with Android 9 or newer.

## Features

- Timetable with daily, agenda and weekly views, Lab FAT days and the week ahead
- Attendance by course, with a calendar, a calculator for goals, classes you can miss and pending OD, and a day planner
- Marks, grades, grade history, exam schedules and a GPA and CGPA planner
- Academic calendar, profile, mentor, and face and biometric entry history
- Course pages with their files, and general and weekend outings with the outing pass
- A change inbox and background sync every 3, 6, 12 or 24 hours that tells you what changed on VTOP
- OTP auto-fill from Gmail with your own Google key, or from a forwarding address
- Docs, a folder you choose for files from VTOP and your own, with the mess menu
- VTOP inside the app
- A choice of connection: direct to VTOP, through a compressing proxy, or server fetch over HTTPS or UDP. If a route through the server fails, the app goes to VTOP directly.
- Connected apps: the browser extension, paired by QR code, and keys for AI apps over MCP

### Browser extension

The vitapmate extension for Chrome and Firefox signs you in to VTOP in the browser with your phone's session, so you skip the CAPTCHA and the OTP. Pair it from **Settings → Connected apps → Pair a browser**. The latest build is on the [extension-latest release](https://github.com/itsAster0/vitap-mate/releases/tag/extension-latest).

## First run

1. Open the app and enter your VTOP username and password.
2. Pick a semester.
3. Wait for the first refresh to finish.
4. In **Settings**, set up background sync, OTP auto-fill, the connection and connected apps if you want them.

The app keeps your VTOP password and the cached pages on the phone. By default it talks to VTOP directly. It uses the vitapmate server (`vmate.aster0.dev`) only for the proxy and server-fetch connections, mail forwarding, push messages and connected apps. **Settings → Server** lists what the server keeps and deletes it.

## Development

The UI is Kotlin with Compose Multiplatform, built with the Kotlin Toolchain (`./kotlin`, no Gradle). Rust (`rust/vtop-ffi`, through UniFFI) does the VTOP sign-in, CAPTCHA, parsing and the local cache. The server is a Cloudflare Worker (`rust/vtop-edge`), an engine on Railway and a UDP gateway; `rust/ARCHITECTURE.md` describes them.

You need:

- [mise](https://mise.jdx.dev), which installs Java, the Kotlin Toolchain and bun at the versions in `mise.toml`
- Rust stable with the `aarch64-linux-android` target, and `cargo-ndk`
- The Android SDK and NDK

Build the Rust library and its Kotlin bindings, then build the app and run the tests:

```bash
mise install
mise run rust-android
./kotlin build
./kotlin test -m core
```

Run `mise run rust-android` again after you change `rust/vtop-ffi`. To run the app on an emulator or a connected phone:

```bash
./kotlin run -d emulator-5554
```

The debug key signs debug builds. A signed release needs the keystore and its `keystore.properties` (`storeFile`, `keyAlias`, `password`) in `keystore/`, which git ignores. `mise run release` writes `app/keystore.properties` from it and builds `build/vitapmate-release.apk`.

### CI

| Workflow | Runs on | Does |
|---|---|---|
| App Build | push to `main`, PRs | builds and tests the app, then replaces the `nightly` release |
| Extension Build | push touching `extension/` | builds the Chrome zip and the signed Firefox `.xpi`, then replaces `extension-latest` |
| App Release | a `v*` tag | builds the signed APK and AAB and uploads the AAB to Play's internal track |

## Troubleshooting

### Android won't install the APK

Allow installs from your browser or file manager when Android asks. If Android reports a conflict with an installed copy, that copy is signed with another key, which can happen with a Play Store copy. Uninstalling it fixes this, but also deletes that copy's saved data.

### Gmail stops filling in the OTP

A Google OAuth project in Testing status issues refresh tokens that expire after seven days. Connect Gmail again from **Settings → OTP auto-fill**.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to contribute and [SECURITY.md](SECURITY.md) to report a vulnerability privately.

## License

See [LICENSE](LICENSE).
