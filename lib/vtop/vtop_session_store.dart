import 'dart:convert';
import 'package:vitapmate/platform/webview/vtop_webview_store.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

const defaultVtopSessionReuseTtl = Duration(minutes: 30);
const vtopSessionReuseTtlSettingKey = 'settings_vtop_session_reuse_ttl_minutes';

Duration vtopSessionReuseTtlFromMinutes(int? minutes) {
  if (minutes == null || minutes <= 0) {
    return defaultVtopSessionReuseTtl;
  }
  return Duration(minutes: minutes);
}

Future<Duration> readVtopSessionReuseTtl() async {
  final storage = await SharedPreferences.getInstance();
  return vtopSessionReuseTtlFromMinutes(
    storage.getInt(vtopSessionReuseTtlSettingKey),
  );
}

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
  if (raw == null || raw.isEmpty) {
    return null;
  }

  try {
    final snapshot = PersistedVtopSession.fromJson(
      jsonDecode(raw) as Map<String, dynamic>,
    );
    // Age counts from the last real login (set by Rust) when known; older
    // snapshots only have the time they were saved.
    final loggedInAt = snapshot.loggedInAt;
    final since = loggedInAt != null
        ? DateTime.fromMillisecondsSinceEpoch(
            loggedInAt.toInt() * 1000,
            isUtc: true,
          )
        : DateTime.fromMillisecondsSinceEpoch(
            snapshot.savedAtEpochMs.toInt(),
            isUtc: true,
          );
    final age = DateTime.now().toUtc().difference(since);
    final ttl = await readVtopSessionReuseTtl();
    final isExpired = age > ttl;
    return StoredVtopSession(
      snapshot: snapshot,
      isExpired: isExpired,
      age: age,
      ttl: ttl,
    );
  } catch (error) {
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
}) {
  final savedAt = DateTime.now().toUtc();
  return exportSessionSnapshot(
    client: client,
    savedAtEpochMs: BigInt.from(savedAt.millisecondsSinceEpoch),
  );
}
