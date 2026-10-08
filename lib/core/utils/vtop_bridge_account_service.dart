import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/utils/app_urls.dart';

const bridgeLinkedSettingKey = 'settings_bridge_linked';
const bridgeServerSignInSettingKey = 'settings_bridge_server_sign_in';
const bridgeLastFcmTokenSettingKey = 'settings_bridge_last_fcm_token';

/// Where the app keeps the secret the bridge hands out on link. Key, phone
/// and delete changes need it, so a leaked access key cannot make them.
abstract class BridgeAppSecretStore {
  Future<String?> read();
  Future<void> write(String secret);
  Future<void> delete();
}

class SecureBridgeAppSecretStore implements BridgeAppSecretStore {
  const SecureBridgeAppSecretStore();

  static const _key = 'bridge_app_secret';
  static const _storage = FlutterSecureStorage();

  @override
  Future<String?> read() => _storage.read(key: _key);

  @override
  Future<void> write(String secret) => _storage.write(key: _key, value: secret);

  @override
  Future<void> delete() => _storage.delete(key: _key);
}

class MemoryBridgeAppSecretStore implements BridgeAppSecretStore {
  MemoryBridgeAppSecretStore([this._secret]);

  String? _secret;

  @override
  Future<String?> read() async => _secret;

  @override
  Future<void> write(String secret) async => _secret = secret;

  @override
  Future<void> delete() async => _secret = null;
}

class BridgeAccountException implements Exception {
  const BridgeAccountException({
    required this.code,
    required this.message,
    required this.statusCode,
  });

  final String code;
  final String message;
  final int statusCode;

  @override
  String toString() => message;
}

class BridgeKey {
  const BridgeKey({required this.id, required this.key, required this.mcpUrl});

  final String id;
  final String key;
  final String mcpUrl;

  /// The MCP URL with the key in it, for agents that only take a URL.
  String get agentUrl => '$mcpUrl/$key';
}

class BridgeKeyInfo {
  const BridgeKeyInfo({
    required this.id,
    required this.label,
    required this.createdAt,
    required this.lastUsedAt,
  });

  final String id;
  final String label;
  final DateTime createdAt;
  final DateTime? lastUsedAt;
}

/// How the bridge serves this account's sessions.
@immutable
class BridgeSettings {
  const BridgeSettings({
    this.alwaysUseVault = true,
    this.phoneWaitSecs = 20,
    this.vaultTtlSecs,
  });

  static const phoneWaitChoices = [10, 20, 45];
  static const vaultTtlChoices = <int?>[86400, 172800, null];

  /// Sign in with the saved credentials straight away, without asking the
  /// phone first.
  final bool alwaysUseVault;

  /// When not [alwaysUseVault]: how long the phone gets before the server
  /// signs in itself.
  final int phoneWaitSecs;

  /// How long saved credentials are kept; `null` keeps them.
  final int? vaultTtlSecs;

  factory BridgeSettings.fromJson(Object? json) {
    if (json is! Map) return const BridgeSettings();
    final wait = json['phoneWaitSecs'];
    final ttl = json['vaultTtlSecs'];
    final always = json['alwaysUseVault'];
    return BridgeSettings(
      alwaysUseVault: always is bool ? always : true,
      phoneWaitSecs: wait is int ? wait : 20,
      vaultTtlSecs: ttl is int ? ttl : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'alwaysUseVault': alwaysUseVault,
    'phoneWaitSecs': phoneWaitSecs,
    'vaultTtlSecs': vaultTtlSecs,
  };

  BridgeSettings copyWith({
    bool? alwaysUseVault,
    int? phoneWaitSecs,
    int? Function()? vaultTtlSecs,
  }) => BridgeSettings(
    alwaysUseVault: alwaysUseVault ?? this.alwaysUseVault,
    phoneWaitSecs: phoneWaitSecs ?? this.phoneWaitSecs,
    vaultTtlSecs: vaultTtlSecs == null ? this.vaultTtlSecs : vaultTtlSecs(),
  );

  @override
  bool operator ==(Object other) =>
      other is BridgeSettings &&
      other.alwaysUseVault == alwaysUseVault &&
      other.phoneWaitSecs == phoneWaitSecs &&
      other.vaultTtlSecs == vaultTtlSecs;

  @override
  int get hashCode => Object.hash(alwaysUseVault, phoneWaitSecs, vaultTtlSecs);
}

class BridgeAccount {
  const BridgeAccount({
    required this.registrationNumber,
    required this.linked,
    required this.savedCredentials,
    required this.phoneLinked,
    required this.thisPhone,
    required this.keys,
    this.settings = const BridgeSettings(),
  });

  final String registrationNumber;
  final bool linked;
  final bool savedCredentials;
  final bool phoneLinked;

  /// This install holds the account's app secret.
  final bool thisPhone;
  final List<BridgeKeyInfo> keys;
  final BridgeSettings settings;

  bool get linkedHere => linked && thisPhone;
}

/// The token is worth sending only for a linked account when it is known and
/// different from the one the bridge already has.
bool shouldSyncFcmToken({
  required bool linked,
  required String? lastSent,
  required String? current,
}) => linked && current != null && current.isNotEmpty && current != lastSent;

/// Runs [run] with the app's VTOP cookies. The app can think it is signed in
/// after VTOP has dropped the session; when the bridge says so, signs in
/// again ([cookies] with `force: true`) and tries once more.
Future<T> withFreshVtopSession<T>({
  required Future<String> Function({required bool force}) cookies,
  required Future<T> Function(String cookies) run,
}) async {
  try {
    return await run(await cookies(force: false));
  } on BridgeAccountException catch (error) {
    if (error.code != 'session_invalid') rethrow;
    return run(await cookies(force: true));
  }
}

DateTime _dateTimeFromUnixSeconds(Object? value) {
  final seconds = value is int ? value : int.tryParse('$value') ?? 0;
  return DateTime.fromMillisecondsSinceEpoch(seconds * 1000, isUtc: true);
}

/// Talks to the vtop-bridge account API. Every call is authenticated with the
/// device's live VTOP cookies; secrets are never logged here.
class VtopBridgeAccountService {
  VtopBridgeAccountService({
    required http.Client client,
    String baseUrl = vtopBridgeBaseUrl,
    BridgeAppSecretStore secrets = const SecureBridgeAppSecretStore(),
  }) : _client = client,
       _baseUrl = baseUrl,
       _secrets = secrets;

  final http.Client _client;
  final String _baseUrl;
  final BridgeAppSecretStore _secrets;

  static const _timeout = Duration(seconds: 15);

  Future<Map<String, dynamic>> _post(
    String path, {
    required String cookies,
    Map<String, dynamic>? body,
  }) async {
    final secret = await _secrets.read();
    final response = await _client
        .post(
          Uri.parse('$_baseUrl$path'),
          headers: {'Content-Type': 'application/json'},
          body: jsonEncode({
            'session': {'cookies': cookies},
            ...?(secret == null ? null : {'appSecret': secret}),
            ...?body,
          }),
        )
        .timeout(_timeout);

    Map<String, dynamic> decoded;
    try {
      final parsed = jsonDecode(response.body);
      decoded = parsed is Map<String, dynamic> ? parsed : const {};
    } catch (_) {
      decoded = const {};
    }

    final statusCode = response.statusCode;
    if (statusCode < 200 || statusCode >= 300) {
      throw BridgeAccountException(
        code: '${decoded['code'] ?? 'http_$statusCode'}',
        message:
            '${decoded['error'] ?? 'Bridge request failed with HTTP $statusCode.'}',
        statusCode: statusCode,
      );
    }
    return decoded;
  }

  Future<void> link({
    required String cookies,
    required String fcmToken,
    Map<String, dynamic>? credentials,
  }) async {
    final body = await _post(
      '/v1/link',
      cookies: cookies,
      body: {
        'fcmToken': fcmToken,
        ...?(credentials == null ? null : {'credentials': credentials}),
      },
    );
    final secret = body['appSecret'];
    if (secret is String && secret.isNotEmpty) await _secrets.write(secret);
  }

  Future<BridgeKey> createKey({
    required String cookies,
    required String label,
  }) async {
    final body = await _post(
      '/v1/keys',
      cookies: cookies,
      body: {'label': label},
    );
    return BridgeKey(
      id: '${body['id'] ?? ''}',
      key: '${body['key'] ?? ''}',
      mcpUrl: '${body['mcpUrl'] ?? ''}',
    );
  }

  Future<BridgeAccount> account({required String cookies}) async {
    final body = await _post('/v1/account', cookies: cookies);
    final keys = <BridgeKeyInfo>[];
    final rawKeys = body['keys'];
    if (rawKeys is List) {
      for (final raw in rawKeys) {
        if (raw is! Map) continue;
        keys.add(
          BridgeKeyInfo(
            id: '${raw['id'] ?? ''}',
            label: '${raw['label'] ?? ''}',
            createdAt: _dateTimeFromUnixSeconds(raw['createdAt']),
            lastUsedAt: raw['lastUsedAt'] == null
                ? null
                : _dateTimeFromUnixSeconds(raw['lastUsedAt']),
          ),
        );
      }
    }
    return BridgeAccount(
      registrationNumber: '${body['registrationNumber'] ?? ''}',
      linked: body['linked'] == true,
      savedCredentials: body['savedCredentials'] == true,
      phoneLinked: body['phoneLinked'] == true,
      thisPhone: body['thisPhone'] == true,
      keys: keys,
      settings: BridgeSettings.fromJson(body['settings']),
    );
  }

  Future<void> revokeKey({required String cookies, required String id}) async {
    await _post('/v1/keys/revoke', cookies: cookies, body: {'id': id});
  }

  Future<void> updateFcmToken({
    required String cookies,
    required String fcmToken,
  }) async {
    await _post(
      '/v1/account/fcm-token',
      cookies: cookies,
      body: {'fcmToken': fcmToken},
    );
  }

  Future<void> forgetCredentials({required String cookies}) async {
    await _post('/v1/account/forget-credentials', cookies: cookies);
  }

  Future<void> updateSettings({
    required String cookies,
    required BridgeSettings settings,
  }) async {
    await _post(
      '/v1/account/settings',
      cookies: cookies,
      body: {'settings': settings.toJson()},
    );
  }

  Future<void> deleteAccount({required String cookies}) async {
    await _post('/v1/account/delete', cookies: cookies);
    await _secrets.delete();
  }
}
