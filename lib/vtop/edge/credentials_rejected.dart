import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/vtop/edge/edge_events.dart';
import 'package:vitapmate/platform/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/vtop/users/saved_credentials.dart';

/// The edge's event when VTOP rejected the password the server kept.
const credentialsRejectedType = 'credentials_rejected';

const _notifiedAtKey = 'settings_credentials_rejected_notified_at';

/// Marks the phone's saved password rejected too (the server's copy came
/// from it), then tells the user to sign in again, at most once an hour
/// (every failed server sign-in sends the event). [reject] defaults to the
/// saved user in storage; the running app passes its own container's.
Future<void> handleCredentialsRejectedEvent(
  EdgeEvent event, {
  Future<void> Function(String title, String body)? notify,
  DateTime Function()? now,
  Future<void> Function()? reject,
}) async {
  await (reject ?? _rejectInStorage)();
  final at = (now ?? DateTime.now)();
  final prefs = SharedPreferencesAsync();
  final last = await prefs.getInt(_notifiedAtKey);
  if (last != null &&
      at.difference(DateTime.fromMillisecondsSinceEpoch(last, isUtc: true)) <
          const Duration(hours: 1)) {
    return;
  }
  await prefs.setInt(_notifiedAtKey, at.toUtc().millisecondsSinceEpoch);
  await (notify ?? FcmCookieNotificationService.showServerAlert)(
    'VTOP rejected your saved password',
    'Open Settings → Server to sign in again.',
  );
}

Future<void> _rejectInStorage() async {
  final container = ProviderContainer();
  try {
    await rejectSavedCredentialsIn(container);
  } finally {
    container.dispose();
  }
}
