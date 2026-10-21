import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/on_device_mode.dart';
import 'package:vitapmate/core/providers/server_url.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/edge/edge_device.dart';
import 'package:vitapmate/core/utils/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';

/// Moves the app to another VITAP Mate server. The install's registration,
/// its link to an account, its access keys and its forwarding address all
/// belong to one server, so they are forgotten and the install registers
/// again with the new one. Nothing is kept per URL: switching back
/// registers afresh too.
class ServerSwitch {
  ServerSwitch({
    required this.devices,
    required this.forwarding,
    required this.appSecret,
    required this.keys,
    required this.saveUrl,
    required this.setLinked,
    required this.onDeviceMode,
    required this.pause,
    required this.resume,
  });

  final EdgeDeviceStore devices;
  final MailForwardingStore forwarding;
  final BridgeAppSecretStore appSecret;
  final BridgeKeyStash keys;

  /// Saves the URL; null returns to the built-in default.
  final Future<void> Function(String? url) saveUrl;
  final Future<void> Function(bool linked) setLinked;
  final Future<bool> Function() onDeviceMode;

  /// Stop and restart the event socket and push token.
  final Future<void> Function() pause;
  final Future<void> Function() resume;

  Future<void> switchTo(String? url) async {
    await pause();
    await devices.delete();
    await forwarding.delete();
    await appSecret.delete();
    await keys.writeAll({});
    await setLinked(false);
    await saveUrl(url);
    // Registering happens with the push token, which resuming sends.
    if (!await onDeviceMode()) await resume();
  }
}

/// Switches to [url] (null: the default) with the app's real stores.
Future<void> switchServer(WidgetRef ref, String? url) async {
  final notifier = ref.read(serverUrlProvider.notifier);
  await ServerSwitch(
    devices: const SecureEdgeDeviceStore(),
    forwarding: const SecureMailForwardingStore(),
    appSecret: const SecureBridgeAppSecretStore(),
    keys: SecureBridgeKeyStash(),
    saveUrl: (url) => url == null ? notifier.reset() : notifier.set(url),
    setLinked: (linked) async {
      final prefs = await ref.read(settingsProvider.future);
      await prefs.setBool(bridgeLinkedSettingKey, linked);
    },
    onDeviceMode: isOnDeviceMode,
    pause: pauseServerFeatures,
    resume: resumeServerFeatures,
  ).switchTo(url);
}
