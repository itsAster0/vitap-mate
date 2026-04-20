import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
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
    final savedAt = DateTime.fromMillisecondsSinceEpoch(
      snapshot.savedAtEpochMs.toInt(),
      isUtc: true,
    );
    final age = DateTime.now().toUtc().difference(savedAt);
    final ttl = await readVtopSessionReuseTtl();
    final isExpired = age > ttl;
    return StoredVtopSession(
      snapshot: snapshot,
      isExpired: isExpired,
      age: age,
      ttl: ttl,
    );
  } catch (error) {
    AppLogger.instance.warning(
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
  final storage = await SharedPreferences.getInstance();
  await storage.remove(_sessionKey(username));
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
