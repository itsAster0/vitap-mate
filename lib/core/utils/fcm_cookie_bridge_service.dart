import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';
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
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/bridge_otp_prompt.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/firebase_options.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/frb_generated.dart';
import 'package:vitapmate/services/class_reminder_notification_service.dart';

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
    unawaited(syncBridgeFcmToken(token));
  });

  try {
    await syncBridgeFcmToken(await FirebaseMessaging.instance.getToken());
  } catch (error, stackTrace) {
    log(
      'Failed to sync the FCM token with the vtop-bridge',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
  }
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

  final wantCredentials = data['wantCredentials'] == '1';

  await FcmCookieNotificationService.showProgress();
  List<Map<String, dynamic>>? cookies;
  String? username;
  String? fcmToken;
  Map<String, dynamic>? credentials;
  String? callbackError;
  try {
    final prepared = await _authenticatedCookieEditorCookies(
      wantCredentials: wantCredentials,
    );
    cookies = prepared.cookies;
    username = prepared.username;
    credentials = prepared.credentials;
  } on BridgeOtpRequired catch (otp) {
    // The phone answers once the user enters the OTP; a late answer is
    // still cached by the bridge.
    await savePendingBridgeOtp(
      PendingBridgeOtp(
        requestId: requestId,
        responseToken: responseToken,
        callbackUrl: callbackUrl,
        wantCredentials: wantCredentials,
        state: otp.state,
        createdAtMs: DateTime.now().millisecondsSinceEpoch,
      ),
    );
    await FcmCookieNotificationService.cancel();
    await showBridgeOtpNotification();
    log('Cookie request $requestId waits for the OTP', name: 'fcm.cookie');
    return;
  } catch (error, stackTrace) {
    callbackError = '$error';
    log(
      'Failed to prepare cookie request $requestId',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
  }

  if (callbackError == null) {
    try {
      fcmToken = await FirebaseMessaging.instance.getToken();
    } catch (_) {
      fcmToken = null;
    }
  }

  try {
    await postCookieCallbackWithRetry(
      callbackUrl: callbackUrl,
      requestId: requestId,
      responseToken: responseToken,
      cookies: cookies,
      error: callbackError,
      username: username,
      fcmToken: fcmToken,
      credentials: credentials,
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

typedef _PreparedCookieBridgePayload = ({
  List<Map<String, dynamic>> cookies,
  String username,
  Map<String, dynamic>? credentials,
});

Future<_PreparedCookieBridgePayload> _authenticatedCookieEditorCookies({
  required bool wantCredentials,
}) async {
  // Headless: nobody can answer an OTP prompt here.
  final container = ProviderContainer(
    overrides: [vtopLoginPromptAllowedProvider.overrideWithValue(false)],
  );
  try {
    final user = await container.read(vtopUserProvider.future);
    final username = user.username?.trim();
    if (username == null || username.isEmpty) {
      throw StateError('No VTOP account is configured on this device.');
    }

    final cookieHeader = await _headlessVtopCookieHeader(container);

    final cookies = cookieEditorCookiesFromHeader(cookieHeader);
    if (cookies.isEmpty) {
      throw StateError('Could not convert VTOP cookies for Cookie-Editor.');
    }

    final credentials = await bridgeCredentialsFor(
      container,
      user,
      wantCredentials: wantCredentials,
    );

    return (cookies: cookies, username: username, credentials: credentials);
  } finally {
    container.dispose();
  }
}

/// The credentials to hand the bridge: only when it asked, the user turned
/// on offline sign-in, and Gmail OTP access is set up.
Future<Map<String, dynamic>?> bridgeCredentialsFor(
  ProviderContainer container,
  VtopUserEntity user, {
  required bool wantCredentials,
}) async {
  if (!wantCredentials) return null;
  EmailOtpOAuthSession? gmail;
  try {
    gmail = await container
        .read(googleEmailOtpAuthServiceProvider)
        .loadSession();
  } catch (_) {
    gmail = null;
  }
  final prefs = await container.read(settingsProvider.future);
  return bridgeCredentialsPayload(
    wantCredentials: wantCredentials,
    serverSignIn: prefs.getBool(bridgeServerSignInSettingKey) ?? false,
    username: user.username?.trim() ?? '',
    password: user is ConfiguredVtopUser ? user.password : '',
    gmail: gmail,
    deleteAfterReading: container.read(emailOtpDeleteAfterReadingProvider),
    sharedClientId: googleOauthClientId,
  );
}

/// Logs in headless (no OTP prompt is possible here) and returns the VTOP
/// session's cookie header.
Future<String> _headlessVtopCookieHeader(ProviderContainer container) async {
  final VtopClient client;
  try {
    // Gmail auto-fetch gets this long; after that the user is asked.
    client = await container
        .read(vClientProvider.notifier)
        .ensureLogin(force: false, promptForOtp: false)
        .timeout(const Duration(seconds: 45));
  } catch (error) {
    final pendingClient = await container.read(vClientProvider.future);
    final state = exportSessionState(client: pendingClient);
    if (state.otpIssuedAt != null) throw BridgeOtpRequired(state);
    rethrow;
  }
  if (!await fetchIsAuth(client: client)) {
    throw StateError('VTOP session is not authenticated after login.');
  }

  final snapshot = createPersistedVtopSessionSnapshot(client: client);
  final cookieHeader = snapshot.cookies?.trim() ?? '';
  if (cookieHeader.isEmpty) {
    throw StateError('Authenticated VTOP session did not include cookies.');
  }
  return cookieHeader;
}

/// Sends the current FCM token to the vtop-bridge account when the user has
/// linked one and the token changed. Runs headless, so it never throws and
/// never logs tokens or cookies.
Future<void> syncBridgeFcmToken(String? token) async {
  final container = ProviderContainer(
    overrides: [vtopLoginPromptAllowedProvider.overrideWithValue(false)],
  );
  try {
    final prefs = await container.read(settingsProvider.future);
    if (!shouldSyncFcmToken(
      linked: prefs.getBool(bridgeLinkedSettingKey) ?? false,
      lastSent: prefs.getString(bridgeLastFcmTokenSettingKey),
      current: token,
    )) {
      return;
    }
    final current = token!;

    final cookieHeader = await _headlessVtopCookieHeader(container);
    final client = http.Client();
    try {
      await VtopBridgeAccountService(
        client: client,
      ).updateFcmToken(cookies: cookieHeader, fcmToken: current);
    } finally {
      client.close();
    }
    await prefs.setString(bridgeLastFcmTokenSettingKey, current);
  } catch (error, stackTrace) {
    log(
      'Failed to sync the FCM token with the vtop-bridge',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
  } finally {
    container.dispose();
  }
}

/// Sends the semester picked in the app to the vtop-bridge account, so
/// agents default to it. Like [syncBridgeFcmToken]: only when linked and
/// changed, headless, never throws.
Future<void> syncBridgeSemester(String? semesterId) async {
  final container = ProviderContainer(
    overrides: [vtopLoginPromptAllowedProvider.overrideWithValue(false)],
  );
  try {
    final prefs = await container.read(settingsProvider.future);
    if (!shouldSyncFcmToken(
      linked: prefs.getBool(bridgeLinkedSettingKey) ?? false,
      lastSent: prefs.getString(bridgeLastSemesterSettingKey),
      current: semesterId,
    )) {
      return;
    }
    final current = semesterId!;
    final cookieHeader = await _headlessVtopCookieHeader(container);
    final client = http.Client();
    try {
      await VtopBridgeAccountService(
        client: client,
      ).updateSemester(cookies: cookieHeader, semesterId: current);
    } finally {
      client.close();
    }
    await prefs.setString(bridgeLastSemesterSettingKey, current);
  } catch (error, stackTrace) {
    log(
      'Failed to sync the semester with the vtop-bridge',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
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

/// Credentials the cookie bridge may hand to the callback server so it can
/// log in on its own. Null unless every required piece is present; secrets
/// are never logged here.
Map<String, dynamic>? bridgeCredentialsPayload({
  required bool wantCredentials,
  required bool serverSignIn,
  required String username,
  required String password,
  required EmailOtpOAuthSession? gmail,
  required bool deleteAfterReading,
  required String sharedClientId,
}) {
  final trimmedUsername = username.trim();
  if (!wantCredentials ||
      !serverSignIn ||
      trimmedUsername.isEmpty ||
      gmail == null ||
      !gmail.hasGmailScope ||
      gmail.refreshToken.isEmpty ||
      password.trim().isEmpty) {
    return null;
  }

  String clientId;
  String? clientSecret;
  switch (gmail.authSource) {
    case EmailOtpAuthSource.personalByok:
      clientId = gmail.oauthClientId?.trim() ?? '';
      if (clientId.isEmpty) return null;
      final secret = gmail.oauthClientSecret?.trim() ?? '';
      clientSecret = secret.isEmpty ? null : secret;
    case EmailOtpAuthSource.sharedBuiltIn:
      if (sharedClientId.isEmpty) return null;
      clientId = sharedClientId;
  }

  return {
    'username': trimmedUsername,
    'password': password,
    'gmail': <String, dynamic>{
      'refresh_token': gmail.refreshToken,
      'client_id': clientId,
      ...?(clientSecret == null ? null : {'client_secret': clientSecret}),
      'delete_after_reading': deleteAfterReading,
    },
  };
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
  String? username,
  String? fcmToken,
  Map<String, dynamic>? credentials,
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
    ...?((username?.trim().isEmpty ?? true) ? null : {'username': username}),
    ...?((fcmToken?.trim().isEmpty ?? true) ? null : {'fcmToken': fcmToken}),
    ...?(credentials == null ? null : {'credentials': credentials}),
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

    // Same handlers as the app's other notifications, so the OTP reply works
    // when only this background handler has started.
    await _notifications.initialize(
      settings: settings,
      onDidReceiveNotificationResponse: (response) {
        unawaited(
          ClassReminderNotificationService.handleNotificationResponse(response),
        );
      },
      onDidReceiveBackgroundNotificationResponse:
          classReminderBackgroundTapHandler,
    );
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
