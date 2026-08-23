<p align="center">
  <img src="assets/icon/icon.png" width="140" alt="vitapmate app icon">
</p>

# vitapmate

vitapmate is an unofficial Android app for students at VIT-AP University. Use it to check attendance, marks, exam schedules, and VTOP without opening the VTOP website each time.

> [!WARNING]
> This project is archived. The Google Play Store version will not receive more updates. New Android builds may still appear here when the source code changes.

## Download

[Open the latest Android builds](https://github.com/itsKryxen/vitap-mate/actions/workflows/android-build.yml?query=branch%3Amain)

1. Open the newest build with a green check mark.
2. Scroll to the **Artifacts** section.
3. Download `vitapmate-universal-apk`.
4. Extract the ZIP file and open the APK on your Android device.
5. Allow installation from your browser or file manager if Android asks.

GitHub may require you to sign in before downloading an artifact.

### Which APK should I download?

Use `vitapmate-universal-apk` unless you know your device architecture. It works on all supported devices.

| APK | Intended device |
| --- | --- |
| `vitapmate-universal-apk` | Any supported Android device |
| `vitapmate-arm64-v8a-apk` | Most modern Android phones |
| `vitapmate-armeabi-v7a-apk` | Older 32-bit Android phones |
| `vitapmate-x86_64-apk` | Android emulators and Intel devices |

## Getting started

1. Install and open vitapmate.
2. Sign in with your VTOP account.
3. Let the first refresh finish. Your attendance, marks, and schedules will then appear in the app.
4. Open Settings if you want to set up Gmail OTP reading.

## What you can do

- Check attendance and course details
- View marks and exam schedules
- Open VTOP inside the app
- Read recently loaded information without refreshing
- Read VTOP OTP emails through an optional Gmail connection

## Gmail OTP reading

Gmail access is optional. The app has a **How to get OAuth credentials** guide that walks you through creating and importing your own Google OAuth client.

Google may show an unverified app warning because vitapmate requests the `gmail.modify` permission. The app uses it to read VTOP OTP messages and to move a read OTP message to Trash when you ask it to.

If your Google project is in Testing mode, Gmail authorization usually expires after seven days. Open the Gmail setup in vitapmate and authorize it again.

Do not share OAuth access tokens, refresh tokens, or imported credentials.

## Privacy

vitapmate processes VTOP pages on your device. It does not send your VTOP user ID or password to a vitapmate server, and it does not store those credentials outside your device.

The app still connects to services you choose to use. These include VTOP, Google for optional Gmail access, and Firebase services used by the app.

## Troubleshooting

### Android says the app cannot be installed

Make sure you extracted the ZIP before opening the APK. You may also need to allow installs from your browser or file manager in Android settings.

If Android reports a signature conflict, uninstall the Play Store version and install the GitHub build again. Uninstalling removes the app's local data.

### The artifact has expired

GitHub keeps these artifacts for 30 days. Return to the [Android builds page](https://github.com/itsKryxen/vitap-mate/actions/workflows/android-build.yml?query=branch%3Amain) and use the newest successful build.

### I downloaded the wrong APK

Download `vitapmate-universal-apk`. The other packages are smaller builds made for specific processor types.

### Gmail stopped reading OTP messages

Open the Gmail setup in vitapmate and connect the account again. Testing-mode Google authorizations usually expire after seven days.

## Project status

vitapmate is no longer maintained as a published Play Store app. The source remains available for reference and community forks, but updates and support are not guaranteed.

vitapmate is an independent project and is not affiliated with VIT-AP University.

## For developers

The app uses Flutter for the interface and Rust for VTOP processing.

Install the required tools:

- [Install Flutter](https://docs.flutter.dev/install)
- [Set up Flutter for Android](https://docs.flutter.dev/platform-integration/android/setup)
- [Install Rust](https://rust-lang.org/tools/install/)

From the project directory, install packages and run the app:

```bash
flutter pub get
flutter run
```

Android release builds need a signing key. Follow Flutter's [Android release guide](https://docs.flutter.dev/deployment/android), then build the APK:

```bash
flutter build apk --release
```

### Optional integrations

Create `.env.json` in the project directory when you need Firebase messaging or the shared Gmail login:

```json
{
  "GOOGLE_OAUTH_CLIENT_ID": "your-android-client-id.apps.googleusercontent.com",
  "FIREBASE_ANDROID_API_KEY": "your-firebase-api-key",
  "FIREBASE_ANDROID_APP_ID": "your-firebase-android-app-id",
  "FIREBASE_MESSAGING_SENDER_ID": "your-firebase-sender-id",
  "FIREBASE_PROJECT_ID": "your-firebase-project-id",
  "FIREBASE_STORAGE_BUCKET": "your-firebase-storage-bucket"
}
```

Use it when running or building:

```bash
flutter run --dart-define-from-file=.env.json
flutter build apk --release --dart-define-from-file=.env.json
```

`GOOGLE_OAUTH_CLIENT_ID` enables the shared Gmail OAuth option. It must be an Android OAuth client ID ending in `.apps.googleusercontent.com`. Configure it for package `com.vitap_pal.app` and add the signing key's SHA fingerprints. Android users can still import their own Desktop OAuth file when this value is absent.

The app stores Gmail access tokens, refresh tokens, and personal OAuth credentials in device secure storage. Do not put tokens in `.env.json`, source control, or GitHub Actions secrets.

The tracked `lib/firebase_options.dart` reads Firebase client settings from these Dart defines. `android/app/google-services.json` remains optional and is ignored by Git.

#### Getting the Firebase values

Run the FlutterFire CLI for your Firebase project:

```bash
flutterfire configure
```

The CLI generates `lib/firebase_options.dart`. You can use it in either of these ways:

1. Copy the Android values from the generated `FirebaseOptions` into `.env.json`:

   - `apiKey` becomes `FIREBASE_ANDROID_API_KEY`
   - `appId` becomes `FIREBASE_ANDROID_APP_ID`
   - `messagingSenderId` becomes `FIREBASE_MESSAGING_SENDER_ID`
   - `projectId` becomes `FIREBASE_PROJECT_ID`
   - `storageBucket` becomes `FIREBASE_STORAGE_BUCKET`

   Restore the repository version of `lib/firebase_options.dart` if the CLI replaced it, then run or build with `--dart-define-from-file=.env.json`.

2. Keep the complete `lib/firebase_options.dart` generated by the CLI. It contains the Firebase values directly, so the Firebase Dart defines are not needed. The file will appear as a local Git modification unless you commit it.
