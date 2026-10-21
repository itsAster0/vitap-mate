/// On-device mode: the app talks to VTOP (and to Google, for Gmail OTPs read
/// on the phone) and nothing else. Every feature that needs the VITAP Mate
/// server is off: connected apps (browser extension, AI
/// agents), OTP mail forwarding, the event channel and push messages, and a
/// server data source.
library;

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';

const onDeviceModeSettingKey = 'settings_on_device_mode';

/// Thrown by a server call made while on-device mode is on.
class OnDeviceModeException implements Exception {
  const OnDeviceModeException();

  static const code = 'on_device_mode';
  static const message = 'On-device mode is on; the server is not used.';

  @override
  String toString() => message;
}

/// The switch as stored, for code without a `ref`: background isolates,
/// push handlers and the service classes.
Future<bool> isOnDeviceMode() async {
  try {
    return await SharedPreferencesAsync().getBool(onDeviceModeSettingKey) ??
        false;
  } catch (_) {
    return false;
  }
}

final onDeviceModeProvider = Provider<bool>((ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool(onDeviceModeSettingKey) ?? false;
});

/// Saves the switch, then stops or restarts the server features that run
/// on their own (event socket, push token).
Future<void> setOnDeviceMode(WidgetRef ref, bool value) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool(onDeviceModeSettingKey, value);
  ref.invalidate(onDeviceModeProvider);
  ref.invalidate(fcmCookieBridgeAvailableProvider);
  if (value) {
    await pauseServerFeatures();
  } else {
    await resumeServerFeatures();
  }
}
