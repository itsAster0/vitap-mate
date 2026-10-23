This is a nightly build, not a stable release. It has every change merged so far, and it may have bugs that a stable release would not.

## Android

Download `vitapmate-universal.apk` on your phone and open it. Android asks you once to allow installs from your browser or file manager. The universal APK works on every supported phone.

The two smaller APKs each hold code for one processor type. `vitapmate-arm64-v8a.apk` fits nearly every phone sold in the last several years. `vitapmate-armeabi-v7a.apk` is for older 32-bit phones. If you are not sure which one you need, use the universal APK.

## iPhone and iPad

`vitapmate.ipa` is unsigned. Install it with AltStore or SideStore, which sign the app with your own Apple ID.

1. Set up AltStore on your computer and your iPhone. The guide at https://faq.altstore.io covers each step.
2. Download `vitapmate.ipa` to your iPhone.
3. Open AltStore, go to the My Apps tab, tap the + button, and choose `vitapmate.ipa`.

A free Apple ID signs apps for 7 days. To keep the app opening, refresh it in AltStore before then while AltServer runs on your computer on the same Wi-Fi network. A free Apple ID can also have only 3 sideloaded apps at a time.

Sharing your VTOP session with the browser extension needs Firebase push messages, which this app supports only on Android, so the iOS build cannot do it.
