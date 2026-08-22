# vitapmate
**vitapmate** is a companion app designed to simplify and enhance student life at VIT-AP University.

## Archive Notice

This app is archived.

> ⚠️ **Note**
>
> There will be **no further updates published to the Google Play Store**.
> I may still add features or fix issues in my free time, but those changes will **not** be pushed to the Play Store.

- Source code remains available for reference and community forks

## ✨ Features

- View your **attendance**, **marks**, and **exam schedules**  
- Open **VTOP instantly** inside the app  
- Say goodbye to **Wi-Fi login limits**  
- Fast and offline-friendly
- Clean, responsive UI with **native performance**

## 🛠️ Tech Stack

- 🖼️ **Flutter** – for building a beautiful, cross-platform UI  
- ⚙️ **Rust** – for fast and secure scraping of student data from VTOP


## 🔐 Privacy First

We take your privacy seriously:
- The app is compiled in GitHub Actions and uploaded to the Play Store within the action itself for transparency.
- **No data leaves your device**  
- All scraping is done locally — even your login credentials stay on your phone  
- We do **not** collect or store your user ID or password — not now, not ever  

Your data is **your** data.

## Build Setup

No environment file is required. The app compiles and runs with every optional integration disabled or using its local default behavior.

### 1. Install Flutter

Follow the official [Flutter installation guide](https://docs.flutter.dev/get-started/install) for your operating system. Install the Android SDK/Android Studio as described there if you plan to build for Android.

After installation, confirm that Flutter and the required platform tools are available:

```bash
flutter doctor
```

Resolve the issues reported by `flutter doctor`, especially the Flutter, Android toolchain, and connected-device checks, before continuing.

### 2. Install project dependencies

From the repository directory, run:

```bash
flutter pub get
```

### 3. Run or build without an environment file

Run the app on a connected device or emulator:

```bash
flutter run
```

Create release builds:

```bash
flutter build apk --release
flutter build appbundle --release
flutter build ios --release
```

The iOS command requires macOS with Xcode configured.

### Optional configuration

Create a local `.env.json` only when you want one or more optional integrations. The file is ignored by Git. Omit unused fields rather than adding empty values.

| Field | Enables | Behavior when omitted |
| --- | --- | --- |
| `GOOGLE_OAUTH_CLIENT_ID` | Shared-key Gmail OAuth fallback | The shared fallback is hidden. Android users can still use the preferred local BYOK flow. |
| `FCM_COOKIE_CALLBACK_URL` | FCM cookie bridge and Chrome extension setup | The listener is not started and the Chrome Extension section is hidden. |

Example with every optional integration configured:

```json
{
  "GOOGLE_OAUTH_CLIENT_ID": "your-android-client-id.apps.googleusercontent.com",
  "FCM_COOKIE_CALLBACK_URL": "https://your-public-backend.example.com/cookie/callback"
}
```

Run or build with optional configuration:

```bash
flutter run --dart-define-from-file=.env.json
flutter build apk --release --dart-define-from-file=.env.json
```

### Optional shared Gmail OAuth fallback

Local BYOK is the preferred Gmail setup and does not require a build-time environment value. To additionally ship the shared fallback:

1. Enable the Gmail API in Google Cloud.
2. Configure the OAuth consent screen with the `gmail.modify` scope.
3. Create an Android OAuth client for package `com.vitap_pal.app`.
4. Add the signing SHA fingerprints and enable the custom URI scheme.
5. Put its client ID in `GOOGLE_OAUTH_CLIENT_ID`.

The Android and iOS builds derive the native redirect scheme only when this value is present. Without it, they use an inert placeholder scheme and the shared option is not shown.

### Personal Gmail BYOK (Android)

No environment value or hosted backend is needed for personal BYOK. The app contains a **How to get OAuth credentials** guide that links directly to the relevant Google Cloud pages. In summary, the user creates a Google Cloud project, enables Gmail API, configures the OAuth audience as **External** with publishing status **Testing**, adds every connecting `@vitapstudent.ac.in` address as a test user, then creates and imports a **Desktop app** OAuth JSON file.

A trusted friend’s Desktop OAuth JSON can also be imported, but the friend’s project must list the connecting college email as a test user. The project owner controls that OAuth client and can revoke it or inspect aggregate usage, so credentials should only be accepted from someone trusted. OAuth access and refresh tokens must never be shared.

Testing authorizations normally expire after seven days. Google may display an unverified-app warning and a broadly worded Gmail permission screen because the app requests `gmail.modify`; the app uses it to read VTOP OTP messages and optionally move a read OTP message to Trash.

While personal authorization is active, the setup page checks secure storage once per second for up to five minutes and updates as soon as the validated token is saved. The loopback listener stays in the app process; it is intentionally not moved to WorkManager because an Android background worker runs separately from the interactive OAuth request and is not a reliable owner for a temporary localhost callback server.

To inspect an Android signing fingerprint:

```bash
keytool -list -v -alias androiddebugkey -keystore ~/.android/debug.keystore -storepass android -keypass android
```

For a release keystore, replace the keystore path and alias with your release signing values.

### Optional FCM extension bridge

Set `FCM_COOKIE_CALLBACK_URL` only if you operate the cookie callback service used by the Chrome extension. When absent, the app skips the FCM cookie listener, token-copy feature, and extension UI entirely.
