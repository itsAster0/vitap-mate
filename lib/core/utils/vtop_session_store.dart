import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

const vtopSessionReuseTtl = Duration(minutes: 120);

class StoredVtopSession {
  const StoredVtopSession({
    required this.snapshot,
    required this.isExpired,
    required this.age,
  });

  final PersistedVtopSession snapshot;
  final bool isExpired;
  final Duration age;
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
    final isExpired = age > vtopSessionReuseTtl;
    return StoredVtopSession(
      snapshot: snapshot,
      isExpired: isExpired,
      age: age,
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
  required Duration ttl,
}) {
  final savedAt = DateTime.now().toUtc();
  final expiresAt = savedAt.add(ttl);
  return exportSessionSnapshot(
    client: client,
    savedAtEpochMs: BigInt.from(savedAt.millisecondsSinceEpoch),
  );
}
