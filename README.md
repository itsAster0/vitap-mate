<p align="center">
  <img src="assets/icon/icon.png" width="140" alt="vitapmate app icon">
</p>

# vitapmate

vitapmate is an unofficial Android client for VIT-AP students. It reads academic data from VTOP and keeps a local copy on the device, so common information remains available between refreshes.

> [!NOTE]
> The app is in low-maintenance mode. The Play Store release is no longer updated, though source changes may still produce new GitHub builds.

## Install

[Open the Android build workflow](https://github.com/itsKryxen/vitap-mate/actions/workflows/android-build.yml?query=branch%3Amain), select the newest successful run, and download an artifact from its **Artifacts** section. GitHub may ask you to sign in.

Most people should use `vitapmate-universal-apk`. Extract the downloaded ZIP, then open the APK on the Android device.

| Artifact | Device |
| --- | --- |
| `vitapmate-universal-apk` | Any supported Android device |
| `vitapmate-arm64-v8a-apk` | Most current Android phones |
| `vitapmate-armeabi-v7a-apk` | Older 32-bit Android phones |
| `vitapmate-x86_64-apk` | Android emulators and Intel devices |

Workflow artifacts expire after 30 days. If a download has expired, use a newer successful run.

## Features

- Timetable with daily, agenda, and weekly views
- Attendance tracking and attendance calculator
- Marks, grades, grade history, and exam schedules
- GPA and projected CGPA calculator
- Face and biometric entry history
- Background sync and automatic data refresh
- Automatic VTOP OTP retrieval from Gmail
- Class reminders, exam reminders, and VTOP change alerts
- Android calendar sync
- In-app VTOP browser and outing shortcuts
- Local document viewer for PDFs, images, spreadsheets, and text files
- Companion Chrome extension for VTOP login

### Timetable

- Daily, agenda, and weekly layouts
- Course, faculty, room, block, slot, and class-time details
- Android calendar sync with a date range, destination calendar, reminder time, and editable title, description, and location templates

### Attendance

- Attendance percentages and attended or missed class counts by course
- A per-course calculator for checking how attending or skipping future classes changes the percentage

### Marks, grades, and exams

- Course-wise marks and assessment breakdowns
- Semester grade view with detailed marks
- Complete grade history
- Exam schedules with date, time, reporting time, venue, room, and seat details when VTOP supplies them
- Exam countdowns
- GPA and projected CGPA calculator with course credits and planned grades
- Face and biometric entry history by date

### VTOP access

- VTOP inside the app, with shortcuts to the course page, general outing, and weekend outing
- Additional web shortcuts for timetable, attendance, academic calendar, assignments, grades, and grade history
- Compact and desktop web views
- Optional Gmail OTP retrieval.
- Optional deletion of an OTP email after it has been read

### Documents

- A local document shelf with a built-in Mess Menu slot
- PDF, image, spreadsheet, text, HTML, JSON, Markdown, XML, and CSV imports
- In-app document viewing with saved reading position and zoom state
- Rename, delete, and recently opened document controls

Supported image extensions are PNG, JPG, JPEG, WebP, GIF, and BMP. Spreadsheet imports accept XLSX, XLSM, XLS, and ODS, though legacy XLS files may need conversion to XLSX before they can be read.

### Sync, alerts, and app controls

- Manual refresh, refresh on page load, and background refresh intervals of 3, 6, 12, or 24 hours
- Local notifications before classes and exams, with adjustable lead times
- Temporary pause for class reminders
- Change alerts for attendance drops, new or revised marks, timetable changes, and exam schedule updates

### Companion Chrome extension

Firebase-enabled Android builds can pair with the vitapmate Chrome extension. The app creates a token used by the extension to request a current authenticated VTOP browser session from the phone. This menu stays hidden when Firebase is unavailable.

## First run

1. Open the app and enter your VTOP username and password.
2. Select a semester.
3. Wait for the first VTOP refresh to finish.
4. Open **Settings** to configure reminders, background sync, Gmail OTP retrieval, or the local CAPTCHA solver.

VTOP credentials and cached academic data are stored on the device. The app connects directly to VTOP. Google, Firebase, and Flagsmith connections are used only when the corresponding integration is configured or enabled.

vitapmate is independent of VIT-AP University.

## Development

The UI is written in Flutter. VTOP login, CAPTCHA recognition, requests, and parsing run through Rust with `flutter_rust_bridge`.

Requirements:

- Flutter stable with Dart 3.11 or newer
- Rust stable
- Java 17 and the Android SDK

Install packages and run the checks:

```bash
flutter pub get
flutter analyze
flutter test
```

The current Android Gradle configuration uses the `release` signing configuration for debug and release builds. Add `android/keystore.properties` and the keystore file referenced by its `storeFile` entry before running the Android app.

```properties
storeFile=../upload-keystore.jks
keyAlias=your-key-alias
password=your-keystore-password
```

Then run or build the app:

```bash
flutter run
flutter build apk --release
```

## Optional build configuration

Plain builds work without hosted integrations. Pass Dart defines through `.env.json` when developing the Gmail shared-login path, Firebase messaging and extension pairing, or remote feature flags.

```json
{
  "GOOGLE_OAUTH_CLIENT_ID": "your-android-client-id.apps.googleusercontent.com",
  "FIREBASE_ANDROID_API_KEY": "your-firebase-api-key",
  "FIREBASE_ANDROID_APP_ID": "your-firebase-android-app-id",
  "FIREBASE_MESSAGING_SENDER_ID": "your-firebase-sender-id",
  "FIREBASE_PROJECT_ID": "your-firebase-project-id",
  "FIREBASE_STORAGE_BUCKET": "your-firebase-storage-bucket",
  "FLAGSMITH_ENV_API_KEY_DEV": "your-development-environment-key",
  "FLAGSMITH_ENV_API_KEY_PROD": "your-production-environment-key",
  "FLAGSMITH_BASE_URI": "https://your-flagsmith-host/api/v1/"
}
```

```bash
flutter run --dart-define-from-file=.env.json
flutter build apk --release --dart-define-from-file=.env.json
```

`GOOGLE_OAUTH_CLIENT_ID` must be an Android OAuth client ID ending in `.apps.googleusercontent.com`. Its Android client must use package name `com.vitap_pal.app` and the certificate fingerprints for the signing key. Without this value, Android users can still import a Desktop OAuth JSON file in the Gmail setup screen.

Firebase needs all five `FIREBASE_ANDROID_*` values shown above. They can be copied from the Android `FirebaseOptions` generated by `flutterfire configure`. The app disables Firebase-dependent controls when that configuration is incomplete.

Flagsmith configuration is optional. When its base URI or the matching environment key is absent, the app enables its feature-flagged screens locally.

## Troubleshooting

### Android cannot install the APK

Extract the workflow artifact before opening the APK. Android may also ask for permission to install apps from the browser or file manager.

A GitHub build and the old Play Store build may use different signatures. If Android reports a signature conflict, removing the installed copy resolves it but also removes that copy's local app data.

### Gmail authorization stops working

Google OAuth projects in Testing status commonly issue refresh tokens that expire after seven days. Reconnect the Gmail account from **Settings > Gmail Autofetch**.

### Calendar sync is unavailable

Calendar sync is Android-only. Grant calendar access and make sure the device has at least one writable calendar.

### A document will not open

Check that its extension appears in the supported document list. For an old `.xls` spreadsheet, save it as `.xlsx` and import the new file.

## Project status

The repository remains open for fixes, documentation work, and community forks. Updates and support are not guaranteed. See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow and [SECURITY.md](SECURITY.md) for private vulnerability reporting.

## License

This project is released under the terms in [LICENSE](LICENSE).
