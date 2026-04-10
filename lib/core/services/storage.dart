import 'dart:convert';
import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/models/account.dart';

class SecureAccountStore {
  SecureAccountStore(this._storage);

  final FlutterSecureStorage _storage;

  static const _activeAccountKey = 'active_account_v2';
  static const _passwordKey = 'active_account_password_v2';
  static const _googleTokensKey = 'google_tokens_v1';

  Future<ActiveAccount?> readActiveAccount() async {
    final raw = await _storage.read(key: _activeAccountKey);
    if (raw == null || raw.isEmpty) return null;
    return ActiveAccount.fromJson(jsonDecode(raw) as Map<String, dynamic>);
  }

  Future<String?> readPassword() {
    return _storage.read(key: _passwordKey);
  }

  Future<void> writeActiveAccount({
    required ActiveAccount account,
    required String password,
  }) async {
    await _storage.write(
      key: _activeAccountKey,
      value: jsonEncode(account.toJson()),
    );
    await _storage.write(key: _passwordKey, value: password);
  }

  Future<void> writeGoogleTokens({
    required String email,
    required String? accessToken,
    String? refreshToken,
    required String? idToken,
    required List<String> scopes,
    DateTime? accessTokenExpiresAt,
    String? tokenType,
  }) async {
    await _storage.write(
      key: _googleTokensKey,
      value: jsonEncode({
        'email': email,
        'accessToken': accessToken,
        'refreshToken': refreshToken,
        'idToken': idToken,
        'scopes': scopes,
        'accessTokenExpiresAt': accessTokenExpiresAt?.toUtc().toIso8601String(),
        'tokenType': tokenType,
        'storedAt': DateTime.now().toUtc().toIso8601String(),
      }),
    );
  }

  Future<Map<String, dynamic>?> readGoogleTokens() async {
    final raw = await _storage.read(key: _googleTokensKey);
    if (raw == null || raw.isEmpty) return null;
    return jsonDecode(raw) as Map<String, dynamic>;
  }

  Future<void> clear() async {
    await _storage.delete(key: _activeAccountKey);
    await _storage.delete(key: _passwordKey);
    await _storage.delete(key: _googleTokensKey);
  }
}

class PreferenceStore {
  PreferenceStore(this._preferences);

  final SharedPreferences _preferences;

  static const _cookieKey = 'vtop_cookie_v2';

  String? readCookie() => _preferences.getString(_cookieKey);

  Future<void> writeCookie(String? cookie) async {
    if (cookie == null || cookie.isEmpty) {
      await _preferences.remove(_cookieKey);
      return;
    }
    await _preferences.setString(_cookieKey, cookie);
  }
}

class JsonCacheStore {
  JsonCacheStore(this._rootDirectory);

  final Directory _rootDirectory;

  File _fileForKey(String key) {
    final safeKey = key.replaceAll(RegExp(r'[^A-Za-z0-9._-]'), '_');
    return File('${_rootDirectory.path}/$safeKey.json');
  }

  Future<dynamic> read(String key) async {
    final file = _fileForKey(key);
    if (!await file.exists()) return null;
    return jsonDecode(await file.readAsString());
  }

  Future<void> write(String key, Object jsonValue) async {
    final file = _fileForKey(key);
    await file.parent.create(recursive: true);
    await file.writeAsString(jsonEncode(jsonValue), flush: true);
  }

  Future<void> clearPrefix(String prefix) async {
    if (!await _rootDirectory.exists()) return;
    await for (final entity in _rootDirectory.list()) {
      if (entity is! File) continue;
      if (entity.uri.pathSegments.last.startsWith(prefix)) {
        await entity.delete();
      }
    }
  }
}
