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

Before compiling, replace the project-specific secrets and OAuth values with
your own.

### Google OAuth

In Google Cloud Console:

1. Enable the `Gmail API` for your project. This app reads OTP emails through
   Gmail readonly access.
2. Configure the OAuth consent screen.
3. Create an `Android` OAuth client with package name `com.vitap_pal.app`.
4. Add your app signing SHA fingerprints to that Android client.
5. In the Android OAuth client, enable `Custom URI scheme` even though Google
   shows a warning saying it is not recommended for Android clients. This app
   uses that redirect flow.
6. Use the generated client ID in the Flutter build.

To get your SHA fingerprints, you can use one of these:

Debug keystore:

```bash
keytool -list -v \
  -alias androiddebugkey \
  -keystore ~/.android/debug.keystore \
  -storepass android \
  -keypass android
```

Release keystore:

```bash
keytool -list -v -keystore /path/to/your-upload-keystore.jks -alias your-key-alias
```

Look for `SHA1` in the output and add it to the Android OAuth
client .

After that, use the generated mobile client ID in the Flutter build:


```bash
flutter run --dart-define-from-file=.env.json
```

Example `.env.json`:

```json
{
  "GOOGLE_OAUTH_CLIENT_ID": "your-client-id.apps.googleusercontent.com"
}
```

Use the Android client ID here, not the web client ID.



### Native OAuth Redirects

Android and iOS derives from the native redirect scheme automatically from
`GOOGLE_OAUTH_CLIENT_ID` during the build, so `.env.json` is the only value you
need to update for OAuth.


## How To Compile

Typical local flow:

```bash
flutter pub get
flutter run --dart-define-from-file=.env.json
```

Release APK / App Bundle:

```bash
flutter build apk --release --dart-define-from-file=.env.json
flutter build appbundle --release --dart-define-from-file=.env.json
```

Release iOS build:

```bash
flutter build ios --release --dart-define-from-file=.env.json
```
