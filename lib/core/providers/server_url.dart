/// The VITAP Mate server the app uses: the built-in [defaultServerUrl], or
/// one the user set on the Server page.
library;

import 'dart:async';
import 'dart:io';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/utils/edge/server_status.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/app_urls.dart';

const serverUrlSettingKey = 'settings_server_url';

/// [input] as a server URL (`scheme://host[:port]`), or null when it is not
/// one: plain `http` only for this machine and private networks, for
/// development; no path, query or fragment.
String? normalizeServerUrl(String input) {
  final uri = Uri.tryParse(input.trim());
  if (uri == null || uri.host.isEmpty || uri.userInfo.isNotEmpty) return null;
  if (uri.hasQuery || uri.hasFragment) return null;
  if (uri.path.isNotEmpty && uri.path != '/') return null;
  final scheme = uri.scheme.toLowerCase();
  if (scheme != 'https' && !(scheme == 'http' && _isLocal(uri.host))) {
    return null;
  }
  final port = uri.hasPort ? ':${uri.port}' : '';
  return '$scheme://${uri.host.toLowerCase()}$port';
}

bool _isLocal(String host) {
  if (host == 'localhost') return true;
  final address = InternetAddress.tryParse(host);
  if (address == null || address.type != InternetAddressType.IPv4) {
    return false;
  }
  final [a, b, ..._] = address.rawAddress;
  return a == 127 ||
      a == 10 ||
      (a == 192 && b == 168) ||
      (a == 172 && b >= 16 && b <= 31);
}

/// Whether [url] answers as a VITAP Mate server (`GET /healthz`).
Future<bool> probeServer(String url, http.Client client) async =>
    (await checkHealth(url, client)).ok;

String _orDefault(String? saved) =>
    (saved == null ? null : normalizeServerUrl(saved)) ?? defaultServerUrl;

/// The server URL as stored, for code without a `ref`: background isolates,
/// push handlers and the service classes.
Future<String> currentServerUrl() async {
  try {
    return _orDefault(
      await SharedPreferencesAsync().getString(serverUrlSettingKey),
    );
  } catch (_) {
    return defaultServerUrl;
  }
}

final serverUrlProvider = NotifierProvider<ServerUrl, String>(ServerUrl.new);

class ServerUrl extends Notifier<String> {
  @override
  String build() {
    final prefs = ref.watch(settingsProvider).value;
    return _orDefault(prefs?.getString(serverUrlSettingKey));
  }

  /// Completes once the saved value has been read.
  Future<void> get ready => ref.read(settingsProvider.future);

  bool get isDefault => state == defaultServerUrl;

  /// Saves [url]; throws [ArgumentError] when it is not a server URL. Call
  /// through `switchServer`, which also forgets the old server's install.
  Future<void> set(String url) async {
    final normalized = normalizeServerUrl(url);
    if (normalized == null) {
      throw ArgumentError.value(url, 'url', 'Not a server URL');
    }
    final prefs = await ref.read(settingsProvider.future);
    await prefs.setString(serverUrlSettingKey, normalized);
    state = normalized;
  }

  Future<void> reset() async {
    final prefs = await ref.read(settingsProvider.future);
    await prefs.remove(serverUrlSettingKey);
    state = defaultServerUrl;
  }
}
