import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/platform/webview/vtop_webview_store.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

const defaultVtopSessionReuseTtl = Duration(minutes: 30);
const vtopSessionReuseTtlSettingKey = 'settings_vtop_session_reuse_ttl_minutes';

Duration vtopSessionReuseTtlFromMinutes(int? minutes) =>
    minutes == null || minutes <= 0
    ? defaultVtopSessionReuseTtl
    : Duration(minutes: minutes);

/// A Unix time in seconds from Rust (login, OTP issue) as UTC.
DateTime utcFromUnixSeconds(BigInt seconds) =>
    DateTime.fromMillisecondsSinceEpoch(seconds.toInt() * 1000, isUtc: true);

class StoredVtopSession {
  const StoredVtopSession({
    required this.snapshot,
    required this.isExpired,
    required this.age,
    required this.ttl,
  });

  final PersistedVtopSession snapshot;
  final bool isExpired;
  final Duration age;
  final Duration ttl;
}

String _sessionKey(String username) => 'vtop_session_${username.toUpperCase()}';

Future<StoredVtopSession?> loadStoredVtopSession(String username) async {
  final storage = await SharedPreferences.getInstance();
  final raw = storage.getString(_sessionKey(username));
  if (raw == null || raw.isEmpty) return null;

  try {
    final snapshot = PersistedVtopSession.fromJson(
      jsonDecode(raw) as Map<String, dynamic>,
    );
    // Age counts from the last real login (set by Rust) when known; older
    // snapshots only have the time they were saved.
    final since = switch (snapshot.loggedInAt) {
      final loggedInAt? => utcFromUnixSeconds(loggedInAt),
      null => DateTime.fromMillisecondsSinceEpoch(
        snapshot.savedAtEpochMs.toInt(),
        isUtc: true,
      ),
    };
    final age = DateTime.now().toUtc().difference(since);
    final ttl = vtopSessionReuseTtlFromMinutes(
      (await SharedPreferences.getInstance()).getInt(
        vtopSessionReuseTtlSettingKey,
      ),
    );
    return StoredVtopSession(
      snapshot: snapshot,
      isExpired: age > ttl,
      age: age,
      ttl: ttl,
    );
  } catch (_) {
    Log.warn(
      'client.session',
      'stored session was unreadable and will be cleared',
    );
    await clearStoredVtopSession(username);
    return null;
  }
}

Future<void> saveStoredVtopSession(PersistedVtopSession snapshot) async {
  final storage = await SharedPreferences.getInstance();
  await storage.setString(_sessionKey(snapshot.username), jsonEncode(snapshot));
}

Future<void> clearStoredVtopSession(String username) async {
  await vtopWebviewStore.clear(username: username);
  final storage = await SharedPreferences.getInstance();
  await storage.remove(_sessionKey(username));
}

/// The `Cookie` header of [client]'s session when it is signed in and has
/// one; null otherwise.
Future<String?> signedInCookies(VtopClient client) async {
  if (!await fetchIsAuth(client: client)) return null;
  final cookies =
      createPersistedVtopSessionSnapshot(client: client).cookies?.trim() ?? '';
  return cookies.isEmpty ? null : cookies;
}

PersistedVtopSession createPersistedVtopSessionSnapshot({
  required VtopClient client,
}) => exportSessionSnapshot(
  client: client,
  savedAtEpochMs: BigInt.from(DateTime.now().toUtc().millisecondsSinceEpoch),
);
