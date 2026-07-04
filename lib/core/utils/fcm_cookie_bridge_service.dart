import 'dart:async';
import 'dart:convert';
import 'dart:developer' show log;

import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
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
const _envCookieCallbackUrl = String.fromEnvironment('FCM_COOKIE_CALLBACK_URL');

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
  try {
    final cookies = await _authenticatedCookieEditorCookies();
    await _postCookieCallback(
      callbackUrl: callbackUrl,
      requestId: requestId,
      responseToken: responseToken,
      cookies: cookies,
    );
  } catch (error, stackTrace) {
    log(
      'Failed to resolve FCM cookie request',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
    await _postCookieCallback(
      callbackUrl: callbackUrl,
      requestId: requestId,
      responseToken: responseToken,
      error: '$error',
    );
  } finally {
    await FcmCookieNotificationService.cancel();
  }
}

String _resolveCookieCallbackUrl(Map<String, dynamic> data) {
  final envCallbackUrl = _envCookieCallbackUrl.trim();
  if (envCallbackUrl.isNotEmpty) return envCallbackUrl;
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

    await container
        .read(vClientProvider.notifier)
        .ensureLogin(force: false, promptForOtp: false);
    final client = await container.read(vClientProvider.future);
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

Future<void> _postCookieCallback({
  required String callbackUrl,
  required String requestId,
  required String responseToken,
  List<Map<String, dynamic>>? cookies,
  String? error,
}) async {
  final response = await http
      .post(
        Uri.parse(callbackUrl),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({
          'requestId': requestId,
          'responseToken': responseToken,
          ...?(cookies == null ? null : {'cookies': cookies}),
          ...?((error?.trim().isEmpty ?? true) ? null : {'error': error}),
        }),
      )
      .timeout(const Duration(seconds: 15));

  if (response.statusCode < 200 || response.statusCode >= 300) {
    throw StateError(
      'Cookie callback failed with HTTP ${response.statusCode}: ${response.body}',
    );
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
}
