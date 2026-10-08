import 'dart:async';
import 'dart:convert';
import 'dart:developer' show log;

import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';
import 'package:vitapmate/src/frb_generated.dart';

/// When a browser or AI agent asks the phone to sign in and VTOP wants an
/// OTP that Gmail auto-fetch cannot supply, the half-finished login is
/// saved and a notification asks for the OTP. The user can type it into the
/// notification, or tap it to use the app's OTP prompt. Either way the
/// phone finishes the login and sends the bridge the cookies.

const bridgeOtpPayloadType = 'bridge_otp';
const bridgeOtpReplyActionId = 'bridge_otp_reply';

const _pendingStorageKey = 'bridge_pending_otp';
const _notificationId = 9003;
const _channelId = 'bridge_otp_v1';
const _staleAfter = Duration(minutes: 10);
const _storage = FlutterSecureStorage();

/// Raised when the headless login stops at VTOP's OTP step.
class BridgeOtpRequired implements Exception {
  const BridgeOtpRequired(this.state);

  /// The login waiting for the OTP.
  final SessionState state;

  @override
  String toString() => 'VTOP needs an OTP';
}

/// A login waiting for its OTP, and the bridge request it answers.
class PendingBridgeOtp {
  const PendingBridgeOtp({
    required this.requestId,
    required this.responseToken,
    required this.callbackUrl,
    required this.wantCredentials,
    required this.state,
    required this.createdAtMs,
  });

  final String requestId;
  final String responseToken;
  final String callbackUrl;
  final bool wantCredentials;
  final SessionState state;
  final int createdAtMs;

  bool isStaleAt(DateTime now) =>
      now.millisecondsSinceEpoch - createdAtMs > _staleAfter.inMilliseconds;

  Map<String, dynamic> toJson() => {
    'requestId': requestId,
    'responseToken': responseToken,
    'callbackUrl': callbackUrl,
    'wantCredentials': wantCredentials,
    'createdAtMs': createdAtMs,
    'state': {
      'cookies': state.cookies,
      'csrfToken': state.csrfToken,
      'registrationNumber': state.registrationNumber,
      'otpIssuedAt': state.otpIssuedAt?.toString(),
      'loggedInAt': state.loggedInAt?.toString(),
    },
  };

  static PendingBridgeOtp? fromJson(Map<String, dynamic> json) {
    final state = json['state'];
    final requestId = json['requestId'];
    final responseToken = json['responseToken'];
    final callbackUrl = json['callbackUrl'];
    if (state is! Map<String, dynamic> ||
        requestId is! String ||
        responseToken is! String ||
        callbackUrl is! String ||
        state['cookies'] is! String) {
      return null;
    }
    BigInt? big(Object? value) =>
        value is String ? BigInt.tryParse(value) : null;
    return PendingBridgeOtp(
      requestId: requestId,
      responseToken: responseToken,
      callbackUrl: callbackUrl,
      wantCredentials: json['wantCredentials'] == true,
      createdAtMs: json['createdAtMs'] is int ? json['createdAtMs'] as int : 0,
      state: SessionState(
        cookies: state['cookies'] as String,
        csrfToken: state['csrfToken'] as String?,
        registrationNumber: state['registrationNumber'] as String?,
        otpIssuedAt: big(state['otpIssuedAt']),
        loggedInAt: big(state['loggedInAt']),
      ),
    );
  }
}

/// What a notification response means for the OTP prompt.
@immutable
class BridgeOtpResponse {
  const BridgeOtpResponse.reply(String this.otp) : kind = 'reply';
  const BridgeOtpResponse.tap() : kind = 'tap', otp = null;
  const BridgeOtpResponse.invalidReply() : kind = 'invalid', otp = null;

  final String kind;
  final String? otp;

  @override
  bool operator ==(Object other) =>
      other is BridgeOtpResponse && other.kind == kind && other.otp == otp;

  @override
  int get hashCode => Object.hash(kind, otp);

  @override
  String toString() => 'BridgeOtpResponse($kind)';
}

/// `null` when the response is not for the OTP prompt.
BridgeOtpResponse? classifyBridgeOtpResponse({
  required String? payload,
  required String? actionId,
  required String? input,
}) {
  var ours = actionId == bridgeOtpReplyActionId;
  if (!ours && payload != null && payload.isNotEmpty) {
    try {
      final parsed = jsonDecode(payload);
      ours = parsed is Map && parsed['type'] == bridgeOtpPayloadType;
    } catch (_) {
      ours = false;
    }
  }
  if (!ours) return null;
  if (actionId != bridgeOtpReplyActionId) return const BridgeOtpResponse.tap();
  final otp = (input ?? '').trim();
  return RegExp(r'^\d{6}$').hasMatch(otp)
      ? BridgeOtpResponse.reply(otp)
      : const BridgeOtpResponse.invalidReply();
}

Future<void> savePendingBridgeOtp(PendingBridgeOtp pending) => _storage.write(
  key: _pendingStorageKey,
  value: jsonEncode(pending.toJson()),
);

Future<PendingBridgeOtp?> loadPendingBridgeOtp() async {
  try {
    final raw = await _storage.read(key: _pendingStorageKey);
    if (raw == null) return null;
    final pending = PendingBridgeOtp.fromJson(
      jsonDecode(raw) as Map<String, dynamic>,
    );
    if (pending == null || pending.isStaleAt(DateTime.now())) {
      await clearPendingBridgeOtp();
      return null;
    }
    return pending;
  } catch (_) {
    return null;
  }
}

Future<void> clearPendingBridgeOtp() async {
  await _storage.delete(key: _pendingStorageKey);
  await FlutterLocalNotificationsPlugin().cancel(id: _notificationId);
}

/// Raised by a tap on the notification while the app is open; the app
/// answers it with its OTP prompt.
final bridgeOtpTapSignal = ValueNotifier<int>(0);

Future<void> showBridgeOtpNotification({String? problem}) async {
  await FcmCookieNotificationService.ensureInitialized();
  final plugin = FlutterLocalNotificationsPlugin();
  await plugin
      .resolvePlatformSpecificImplementation<
        AndroidFlutterLocalNotificationsPlugin
      >()
      ?.createNotificationChannel(
        const AndroidNotificationChannel(
          _channelId,
          'Sign-in OTP',
          description: 'Asks for the VTOP OTP when another device signs in',
          importance: Importance.high,
        ),
      );
  await plugin.show(
    id: _notificationId,
    title: problem ?? 'VTOP needs your OTP',
    body:
        'Enter the OTP from your VIT email to finish signing in your browser or AI agent.',
    payload: jsonEncode({'type': bridgeOtpPayloadType}),
    notificationDetails: const NotificationDetails(
      android: AndroidNotificationDetails(
        _channelId,
        'Sign-in OTP',
        channelDescription:
            'Asks for the VTOP OTP when another device signs in',
        importance: Importance.high,
        priority: Priority.high,
        category: AndroidNotificationCategory.message,
        autoCancel: false,
        timeoutAfter: 1000 * 60 * 10,
        actions: [
          AndroidNotificationAction(
            bridgeOtpReplyActionId,
            'Enter OTP',
            inputs: [AndroidNotificationActionInput(label: '6-digit OTP')],
            cancelNotification: false,
          ),
        ],
      ),
    ),
  );
}

/// Handles a notification response if it is for the OTP prompt.
Future<bool> handleBridgeOtpNotificationResponse(
  NotificationResponse response,
) async {
  final kind = classifyBridgeOtpResponse(
    payload: response.payload,
    actionId: response.actionId,
    input: response.input,
  );
  if (kind == null) return false;
  switch (kind.kind) {
    case 'reply':
      await submitPendingBridgeOtp(kind.otp!);
    case 'invalid':
      await showBridgeOtpNotification(problem: 'Enter the 6-digit OTP');
    default:
      bridgeOtpTapSignal.value++;
  }
  return true;
}

/// Finishes the saved login with [otp] (from the notification) and sends
/// the cookies to the bridge. Runs headless.
Future<void> submitPendingBridgeOtp(String otp) async {
  final pending = await loadPendingBridgeOtp();
  if (pending == null) {
    await clearPendingBridgeOtp();
    return;
  }
  try {
    await RustLib.init();
  } catch (_) {
    // Already initialised in this isolate.
  }
  final container = ProviderContainer(
    overrides: [vtopLoginPromptAllowedProvider.overrideWithValue(false)],
  );
  try {
    final client = await container.read(vClientProvider.future);
    vtopClientResumeSession(client: client, session: pending.state);
    try {
      await vtopClientSubmitSecurityOtp(client: client, otpCode: otp);
    } catch (error) {
      log('Bridge OTP rejected', name: 'fcm.cookie', error: error);
      await showBridgeOtpNotification(problem: 'That OTP did not work');
      return;
    }
    await _answerPending(container, client, pending);
  } catch (error, stackTrace) {
    log(
      'Finishing the bridge OTP login failed',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
    await showBridgeOtpNotification(problem: 'Sign-in failed, try again');
  } finally {
    container.dispose();
  }
}

/// Answers a tap: resumes the saved login in the app and shows the app's
/// OTP prompt; when it succeeds, sends the cookies to the bridge.
Future<void> answerPendingBridgeOtpInApp(ProviderContainer container) async {
  final pending = await loadPendingBridgeOtp();
  if (pending == null) return;
  try {
    final client = await container.read(vClientProvider.future);
    vtopClientResumeSession(client: client, session: pending.state);
    await container
        .read(vtopOtpChallengeProvider.notifier)
        .requestOtp(
          client: client,
          message: 'Enter the OTP to sign in your browser or AI agent.',
          otpRequiredAt: pending.state.otpIssuedAt == null
              ? null
              : DateTime.fromMillisecondsSinceEpoch(
                  pending.state.otpIssuedAt!.toInt() * 1000,
                  isUtc: true,
                ),
        );
    await _answerPending(container, client, pending);
  } catch (error, stackTrace) {
    log(
      'Answering the bridge OTP in the app failed',
      name: 'fcm.cookie',
      error: error,
      stackTrace: stackTrace,
    );
  }
}

Future<void> _answerPending(
  ProviderContainer container,
  VtopClient client,
  PendingBridgeOtp pending,
) async {
  final cookieHeader =
      createPersistedVtopSessionSnapshot(client: client).cookies?.trim() ?? '';
  if (cookieHeader.isEmpty) throw StateError('No VTOP cookies after the OTP.');
  final user = await container.read(vtopUserProvider.future);
  final username = user.username?.trim();
  final credentials = await bridgeCredentialsFor(
    container,
    user,
    wantCredentials: pending.wantCredentials,
  );
  String? fcmToken;
  try {
    fcmToken = await ensureFirebaseReady()
        ? await FirebaseMessaging.instance.getToken()
        : null;
  } catch (_) {
    fcmToken = null;
  }
  await postCookieCallbackWithRetry(
    callbackUrl: pending.callbackUrl,
    requestId: pending.requestId,
    responseToken: pending.responseToken,
    cookies: cookieEditorCookiesFromHeader(cookieHeader),
    username: username,
    fcmToken: fcmToken,
    credentials: credentials,
  );
  await clearPendingBridgeOtp();
  log('Bridge OTP login completed', name: 'fcm.cookie');
}
