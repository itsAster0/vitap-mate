import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/simple.dart';

/// Developer → Inspect Traffic. On, the app's HTTP (Dart and Rust) goes
/// through the phone's Wi-Fi proxy and trusts the user-installed CAs, so a
/// debugging proxy (Proxyman, mitmproxy) can read it. Off by default: with
/// it on, whoever owns a user CA can read VTOP passwords and cookies.
abstract final class TrafficInspection {
  static const settingKey = 'settings_inspect_traffic';

  /// The proxy (`host:port`) and CAs, set by [init] when on.
  static ({String? proxy, List<String> caPems})? current;

  /// Reads the setting and, when on, the phone's proxy and user CAs; call
  /// once at launch, after the Rust runtime starts and before any request.
  static Future<void> init() async {
    try {
      if (await SharedPreferencesAsync().getBool(settingKey) != true) return;
      final found = await const MethodChannel(
        'vitapmate/network',
      ).invokeMapMethod<String, Object?>('trafficInspection');
      final proxy = found?['proxy'] as String?;
      final caPems = [...?(found?['cas'] as List?)?.cast<String>()];
      current = (proxy: proxy, caPems: caPems);
      setTrafficInspection(proxy: proxy, caPems: caPems);
      Log.warn(
        'net',
        'traffic inspection on: proxy ${proxy ?? 'none'}, '
            '${caPems.length} user CAs',
      );
    } catch (error) {
      // Background isolates have no channel; they run uninspected.
      Log.info('net', 'traffic inspection unavailable: $error');
    }
  }
}
