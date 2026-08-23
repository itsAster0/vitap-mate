import 'dart:async';
import 'dart:convert';
import 'dart:developer' show log;

import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/firebase_options.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:vitapmate/src/frb_generated.dart';

const _cookieRequestType = 'vtop_cookie_request';
const _vtopDomain = 'vtop.vitap.ac.in';

bool get fcmCookieBridgePlatformSupported =>
    !kIsWeb && defaultTargetPlatform == TargetPlatform.android;

final fcmCookieBridgeAvailableProvider = FutureProvider<bool>((ref) async {
  if (!fcmCookieBridgePlatformSupported) return false;
  return ensureFirebaseReady();
});

bool _foregroundListenerStarted = false;

Future<bool> ensureFirebaseReady() async {
  if (Firebase.apps.isNotEmpty) return true;

  try {
    await Firebase.initializeApp(
      options: DefaultFirebaseOptions.currentPlatform,
    );
    return true;
  } on UnsupportedError {
    try {
      await Firebase.initializeApp();
      return true;
    } catch (error, stackTrace) {
      log(
        'Firebase initialization is unavailable on this platform',
        name: 'fcm.cookie',
        error: error,
        stackTrace: stackTrace,
      );
      return false;
    }
  } catch (error, stackTrace) {
    log(
      'Firebase initialization failed',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
    return false;
  }
}

Future<String?> getFcmTokenForCopy() async {
  if (!await ensureFirebaseReady()) return null;
  final messaging = FirebaseMessaging.instance;
  await messaging.requestPermission();
  return messaging.getToken();
}

Future<String?> resetFcmTokenForCopy() async {
  if (!await ensureFirebaseReady()) return null;
  final messaging = FirebaseMessaging.instance;
  await messaging.requestPermission();
  await messaging.deleteToken();
  return messaging.getToken();
}

void startVtopCookieBridgeListener() {
  if (_foregroundListenerStarted) return;
  _foregroundListenerStarted = true;
  unawaited(_startVtopCookieBridgeListener());
}

Future<void> _startVtopCookieBridgeListener() async {
  if (!await ensureFirebaseReady()) return;

  FirebaseMessaging.onMessage.listen((message) {
    unawaited(handleVtopCookieBridgeMessage(message.data));
  });

  FirebaseMessaging.instance.onTokenRefresh.listen((token) {
    log('FCM token refreshed (${token.length} chars)', name: 'fcm.cookie');
  });
}

@pragma('vm:entry-point')
Future<void> vtopCookieBridgeBackgroundHandler(RemoteMessage message) async {
  WidgetsFlutterBinding.ensureInitialized();
  if (!await ensureFirebaseReady()) return;
  await RustLib.init();
  await handleVtopCookieBridgeMessage(message.data);
}

Future<void> handleVtopCookieBridgeMessage(Map<String, dynamic> data) async {
  final type = '${data['type'] ?? ''}';
  if (type != _cookieRequestType) return;

  final requestId = '${data['requestId'] ?? ''}'.trim();
  final responseToken = '${data['responseToken'] ?? ''}'.trim();
  final callbackUrl = _resolveCookieCallbackUrl(data);
  if (requestId.isEmpty || responseToken.isEmpty || callbackUrl.isEmpty) {
    log('Ignoring malformed FCM cookie request', name: 'fcm.cookie');
    return;
  }

  await FcmCookieNotificationService.showProgress();
  List<Map<String, dynamic>>? cookies;
  String? callbackError;
  try {
    cookies = await _authenticatedCookieEditorCookies();
  } catch (error, stackTrace) {
    callbackError = '$error';
    log(
      'Failed to prepare cookie request $requestId',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
  }

  try {
    await postCookieCallbackWithRetry(
      callbackUrl: callbackUrl,
      requestId: requestId,
      responseToken: responseToken,
      cookies: cookies,
      error: callbackError,
    );
    log('Completed cookie request $requestId', name: 'fcm.cookie');
    await FcmCookieNotificationService.cancel();
  } catch (error, stackTrace) {
    log(
      'Cookie callback failed for request $requestId',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
    await FcmCookieNotificationService.showFailure();
  }
}

String _resolveCookieCallbackUrl(Map<String, dynamic> data) {
  return resolveCookieCallbackUrlForTest(data);
}

String resolveCookieCallbackUrlForTest(Map<String, dynamic> data) {
  return '${data['callbackUrl'] ?? ''}'.trim();
}

Future<List<Map<String, dynamic>>> _authenticatedCookieEditorCookies() async {
  final container = ProviderContainer();
  try {
    final user = await container.read(vtopUserProvider.future);
    final username = user.username?.trim();
    if (username == null || username.isEmpty) {
      throw StateError('No VTOP account is configured on this device.');
    }

    final client = await container
        .read(vClientProvider.notifier)
        .ensureLogin(force: false, promptForOtp: false);
    if (!await fetchIsAuth(client: client)) {
      throw StateError('VTOP session is not authenticated after login.');
    }

    final snapshot = createPersistedVtopSessionSnapshot(client: client);
    final cookieHeader = snapshot.cookies?.trim() ?? '';
    if (cookieHeader.isEmpty) {
      throw StateError('Authenticated VTOP session did not include cookies.');
    }

    final cookies = cookieEditorCookiesFromHeader(cookieHeader);
    if (cookies.isEmpty) {
      throw StateError('Could not convert VTOP cookies for Cookie-Editor.');
    }
    return cookies;
  } finally {
    container.dispose();
  }
}

List<Map<String, dynamic>> cookieEditorCookiesFromHeader(String cookieHeader) {
  final parts = cookieHeader.split(';');
  final cookies = <Map<String, dynamic>>[];

  for (var i = 0; i < parts.length; i++) {
    final part = parts[i].trim();
    if (part.isEmpty) continue;
    final eq = part.indexOf('=');
    if (eq <= 0) continue;
    final name = part.substring(0, eq).trim();
    final value = part.substring(eq + 1).trim();
    if (name.isEmpty || value.isEmpty) continue;

    cookies.add({
      'domain': _vtopDomain,
      'hostOnly': true,
      'httpOnly': false,
      'name': name,
      'path': '/',
      'sameSite': 'unspecified',
      'secure': true,
      'session': true,
      'storeId': '0',
      'value': value,
      'id': i + 1,
    });
  }

  return cookies;
}

String cookieEditorJsonFromHeader(String cookieHeader) {
  return const JsonEncoder.withIndent(
    '  ',
  ).convert(cookieEditorCookiesFromHeader(cookieHeader));
}

class CookieCallbackException implements Exception {
  const CookieCallbackException(this.statusCode, {required this.retryable});

  final int statusCode;
  final bool retryable;

  @override
  String toString() => 'Cookie callback failed with HTTP $statusCode.';
}

bool isRetryableCookieCallbackStatus(int statusCode) =>
    statusCode == 408 || statusCode == 429 || statusCode >= 500;

Future<void> postCookieCallbackWithRetry({
  required String callbackUrl,
  required String requestId,
  required String responseToken,
  List<Map<String, dynamic>>? cookies,
  String? error,
  http.Client? client,
  Future<void> Function(Duration duration)? delay,
}) async {
  final ownedClient = client == null;
  final httpClient = client ?? http.Client();
  final wait = delay ?? Future<void>.delayed;
  final body = jsonEncode({
    'requestId': requestId,
    'responseToken': responseToken,
    ...?(cookies == null ? null : {'cookies': cookies}),
    ...?((error?.trim().isEmpty ?? true) ? null : {'error': error}),
  });

  try {
    Object? lastError;
    for (var attempt = 1; attempt <= 3; attempt++) {
      try {
        final response = await httpClient
            .post(
              Uri.parse(callbackUrl),
              headers: {'Content-Type': 'application/json'},
              body: body,
            )
            .timeout(const Duration(seconds: 15));
        if (response.statusCode >= 200 && response.statusCode < 300) return;

        final exception = CookieCallbackException(
          response.statusCode,
          retryable: isRetryableCookieCallbackStatus(response.statusCode),
        );
        if (!exception.retryable) throw exception;
        lastError = exception;
      } on CookieCallbackException catch (exception) {
        if (!exception.retryable) rethrow;
        lastError = exception;
      } on TimeoutException catch (exception) {
        lastError = exception;
      } on http.ClientException catch (exception) {
        lastError = exception;
      }

      if (attempt < 3) {
        log(
          'Retrying cookie callback for request $requestId after attempt $attempt',
          name: 'fcm.cookie',
        );
        await wait(Duration(seconds: 1 << (attempt - 1)));
      }
    }
    throw StateError('Cookie callback failed after 3 attempts: $lastError');
  } finally {
    if (ownedClient) httpClient.close();
  }
}

class FcmCookieNotificationService {
  static final FlutterLocalNotificationsPlugin _notifications =
      FlutterLocalNotificationsPlugin();

  static const int _notificationId = 9002;
  static const String _channelId = 'fcm_cookie_bridge_v1';
  static bool _initialized = false;

  static const AndroidNotificationChannel _channel = AndroidNotificationChannel(
    _channelId,
    'Cookie bridge',
    description: 'VTOP cookie bridge requests',
    importance: Importance.min,
    playSound: false,
    enableVibration: false,
  );

  static Future<void> ensureInitialized() async {
    if (_initialized) return;

    const androidSettings = AndroidInitializationSettings(
      '@mipmap/launcher_icon',
    );
    const settings = InitializationSettings(android: androidSettings);

    await _notifications.initialize(settings: settings);
    await _notifications
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >()
        ?.createNotificationChannel(_channel);

    _initialized = true;
  }

  static Future<void> showProgress() async {
    await ensureInitialized();
    await _notifications.show(
      id: _notificationId,
      title: 'VITAP Mate',
      body: 'Fetching VTOP cookies…',
      notificationDetails: const NotificationDetails(
        android: AndroidNotificationDetails(
          _channelId,
          'Cookie bridge',
          channelDescription: 'VTOP cookie bridge requests',
          importance: Importance.min,
          priority: Priority.min,
          ongoing: true,
          indeterminate: true,
          showProgress: true,
          silent: true,
          playSound: false,
          enableVibration: false,
          onlyAlertOnce: true,
          timeoutAfter: 1000 * 60 * 2,
        ),
      ),
    );
  }

  static Future<void> cancel() async {
    await ensureInitialized();
    await _notifications.cancel(id: _notificationId);
  }

  static Future<void> showFailure() async {
    await ensureInitialized();
    await _notifications.show(
      id: _notificationId,
      title: 'Browser sign-in failed',
      body: 'Open VITAP Mate, copy a fresh Token, and try again.',
      notificationDetails: const NotificationDetails(
        android: AndroidNotificationDetails(
          _channelId,
          'Cookie bridge',
          channelDescription: 'VTOP cookie bridge requests',
          importance: Importance.defaultImportance,
          priority: Priority.defaultPriority,
          autoCancel: true,
        ),
      ),
    );
  }
}
