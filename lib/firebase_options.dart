import 'package:firebase_core/firebase_core.dart' show FirebaseOptions;
import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform, kIsWeb;

/// Firebase client configuration supplied through Dart defines.
///
/// Local builds can load the ignored `.env.json` file with
/// `--dart-define-from-file=.env.json`.
class DefaultFirebaseOptions {
  static const _androidApiKey = String.fromEnvironment(
    'FIREBASE_ANDROID_API_KEY',
  );
  static const _androidAppId = String.fromEnvironment(
    'FIREBASE_ANDROID_APP_ID',
  );
  static const _messagingSenderId = String.fromEnvironment(
    'FIREBASE_MESSAGING_SENDER_ID',
  );
  static const _projectId = String.fromEnvironment('FIREBASE_PROJECT_ID');
  static const _storageBucket = String.fromEnvironment(
    'FIREBASE_STORAGE_BUCKET',
  );

  static FirebaseOptions get currentPlatform {
    if (kIsWeb) {
      throw UnsupportedError('Firebase is not configured for web.');
    }

    return switch (defaultTargetPlatform) {
      TargetPlatform.android => android,
      _ => throw UnsupportedError(
        'Firebase Dart options are not configured for this platform.',
      ),
    };
  }

  static FirebaseOptions get android {
    if (_androidApiKey.isEmpty ||
        _androidAppId.isEmpty ||
        _messagingSenderId.isEmpty ||
        _projectId.isEmpty ||
        _storageBucket.isEmpty) {
      throw UnsupportedError('Firebase Android Dart defines are missing.');
    }

    return const FirebaseOptions(
      apiKey: _androidApiKey,
      appId: _androidAppId,
      messagingSenderId: _messagingSenderId,
      projectId: _projectId,
      storageBucket: _storageBucket,
    );
  }
}
