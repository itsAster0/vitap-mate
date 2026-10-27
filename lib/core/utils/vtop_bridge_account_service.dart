import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/providers/on_device_mode.dart';
import 'package:vitapmate/core/utils/edge/edge_device.dart';
import 'package:vitapmate/core/providers/server_url.dart';

const bridgeLinkedSettingKey = 'settings_bridge_linked';
const bridgeServerSignInSettingKey = 'settings_bridge_server_sign_in';
const bridgeLastFcmTokenSettingKey = 'settings_bridge_last_fcm_token';
const bridgeLastSemesterSettingKey = 'settings_bridge_last_semester';

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

/// Keys this phone created. The bridge keeps only their hashes, so this is
/// the only place a key can be shown again.
abstract class BridgeKeyStash {
  Future<Map<String, BridgeKey>> readAll();
  Future<void> writeAll(Map<String, BridgeKey> keys);

  Future<BridgeKey?> read(String id) async => (await readAll())[id];

  Future<void> save(BridgeKey key) async =>
      writeAll({...await readAll(), key.id: key});

  Future<void> remove(String id) async =>
      writeAll({...await readAll()}..remove(id));

  /// Drops keys the bridge no longer has (revoked elsewhere, or by a
  /// relink).
  Future<void> retainOnly(Set<String> ids) async {
    final all = await readAll();
    if (all.keys.every(ids.contains)) return;
    await writeAll({...all}..removeWhere((id, _) => !ids.contains(id)));
  }
}

class SecureBridgeKeyStash extends BridgeKeyStash {
  SecureBridgeKeyStash();

  static const _key = 'bridge_key_stash';
  static const _storage = FlutterSecureStorage();

  @override
  Future<Map<String, BridgeKey>> readAll() async {
    try {
      final raw = await _storage.read(key: _key);
      if (raw == null) return {};
      final decoded = jsonDecode(raw);
      if (decoded is! Map) return {};
      return {
        for (final entry in decoded.entries)
          if (entry.value is Map)
            '${entry.key}': BridgeKey(
              id: '${entry.key}',
              key: '${entry.value['key'] ?? ''}',
              mcpUrl: '${entry.value['mcpUrl'] ?? ''}',
            ),
      };
    } catch (_) {
      return {};
    }
  }

  @override
  Future<void> writeAll(Map<String, BridgeKey> keys) async {
    if (keys.isEmpty) {
      await _storage.delete(key: _key);
      return;
    }
    await _storage.write(
      key: _key,
      value: jsonEncode({
        for (final key in keys.values)
          key.id: {'key': key.key, 'mcpUrl': key.mcpUrl},
      }),
    );
  }
}

class MemoryBridgeKeyStash extends BridgeKeyStash {
  Map<String, BridgeKey> _keys = {};

  @override
  Future<Map<String, BridgeKey>> readAll() async => {..._keys};

  @override
  Future<void> writeAll(Map<String, BridgeKey> keys) async => _keys = {...keys};
}

class BridgeAccountException implements Exception, UnknownDeviceError {
  const BridgeAccountException({
    required this.code,
    required this.message,
    required this.statusCode,
  });

  final String code;
  final String message;
  final int statusCode;

  @override
  bool get unknownDevice => isUnknownDevice(code, message, statusCode);

  @override
  String toString() => message;
}

class BridgeKey {
  const BridgeKey({required this.id, required this.key, required this.mcpUrl});

  final String id;
  final String key;
  final String mcpUrl;

  /// The header MCP agents send the key in.
  String get authHeader => 'Authorization: Bearer $key';

  /// The MCP URL with the key in it, for agents that only take a URL.
  String get agentUrl => '$mcpUrl/$key';

  /// Adds the MCP server to Claude Code with the key in the header.
  String get claudeCodeCommand =>
      'claude mcp add --transport http vtop $mcpUrl '
      '--header "Authorization: Bearer $key"';
}

class BridgeKeyInfo {
  const BridgeKeyInfo({
    required this.id,
    required this.label,
    required this.createdAt,
    required this.lastUsedAt,
    this.phoneId,
    this.outings = false,
  });

  final String id;
  final String label;
  final DateTime createdAt;
  final DateTime? lastUsedAt;

  /// Whether AI agents may apply for and cancel outings with it.
  final bool outings;

  /// The phone that created it ([BridgePhone.id]), the only one asked for
  /// its sessions; null for keys from before, which ask every phone.
  final String? phoneId;
}

/// A phone linked to the account; each is asked for sessions.
class BridgePhone {
  const BridgePhone({
    required this.id,
    required this.label,
    required this.addedAt,
    required this.seenAt,
    required this.thisPhone,
  });

  final String id;

  /// The name the phone gave itself ("OnePlus CPH2573"), if any.
  final String? label;
  final DateTime addedAt;
  final DateTime seenAt;
  final bool thisPhone;
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
    this.phones = const [],
    this.phoneCount = 0,
    this.settings = const BridgeSettings(),
    this.semesterId,
    this.vaultSavedAt,
    this.credentialsRejectedAt,
  });

  final String registrationNumber;
  final bool linked;
  final bool savedCredentials;
  final bool phoneLinked;

  /// This install is one of the linked phones.
  final bool thisPhone;
  final List<BridgeKeyInfo> keys;

  /// The linked phones; listed only to a linked phone.
  final List<BridgePhone> phones;

  /// How many phones are linked, shown to any phone.
  final int phoneCount;
  final BridgeSettings settings;

  /// The semester the bridge has from this app; agents default to it.
  final String? semesterId;

  /// When the usable saved credentials were sent.
  final DateTime? vaultSavedAt;

  /// When VTOP last refused the saved credentials (they are dropped then).
  final DateTime? credentialsRejectedAt;

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
    String? baseUrl,
    BridgeAppSecretStore secrets = const SecureBridgeAppSecretStore(),
    BridgeKeyStash? keys,
  }) : _client = client,
       _baseUrl = baseUrl,
       _secrets = secrets,
       _keys = keys ?? SecureBridgeKeyStash();

  final http.Client _client;

  /// Fixed for tests; otherwise the server URL in use at each call.
  final String? _baseUrl;
  final BridgeAppSecretStore _secrets;
  final BridgeKeyStash _keys;

  /// A key this phone created, to copy again; `null` when it was created
  /// elsewhere or before keys were kept.
  Future<BridgeKey?> savedKey(String id) => _keys.read(id);

  static const _timeout = Duration(seconds: 15);

  Future<String> _base() async => _baseUrl ?? await currentServerUrl();

  Future<void> _ensureServerAllowed() async {
    if (await isOnDeviceMode()) {
      throw const BridgeAccountException(
        code: OnDeviceModeException.code,
        message: OnDeviceModeException.message,
        statusCode: 0,
      );
    }
  }

  Future<Map<String, dynamic>> _post(
    String path, {
    required String cookies,
    Map<String, dynamic>? body,
  }) async {
    await _ensureServerAllowed();
    final secret = await _secrets.read();
    final response = await _client
        .post(
          Uri.parse('${await _base()}$path'),
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

  /// Links this phone next to any already linked; [label] names it in the
  /// phone list.
  Future<void> link({
    required String cookies,
    required String fcmToken,
    Map<String, dynamic>? credentials,
    String? semesterId,
    EdgeDevice? device,
    String? label,
  }) async {
    final body = await _post(
      '/v1/link',
      cookies: cookies,
      body: {
        'fcmToken': fcmToken,
        // Session requests then arrive as events of this install.
        ...?(device == null
            ? null
            : {'deviceId': device.id, 'deviceSecret': device.secret}),
        ...?(semesterId == null ? null : {'semesterId': semesterId}),
        ...?(credentials == null ? null : {'credentials': credentials}),
        ...?(label == null ? null : {'label': label}),
      },
    );
    final secret = body['appSecret'];
    if (secret is String && secret.isNotEmpty) await _secrets.write(secret);
  }

  /// VTOP turned the saved password down on this phone: the server stops
  /// signing in with its copy too. Needs no session (there is none now);
  /// the app secret proves the phone. Does nothing when not linked.
  Future<void> reportCredentialsRejected({
    required String registrationNumber,
  }) async {
    if (await isOnDeviceMode()) return;
    final secret = await _secrets.read();
    if (secret == null || registrationNumber.trim().isEmpty) return;
    final response = await _client
        .post(
          Uri.parse('${await _base()}/v1/account/credentials-rejected'),
          headers: {'Content-Type': 'application/json'},
          body: jsonEncode({
            'registrationNumber': registrationNumber.trim(),
            'appSecret': secret,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode >= 300) {
      throw BridgeAccountException(
        code: 'http_${response.statusCode}',
        message: 'The server did not take the report.',
        statusCode: response.statusCode,
      );
    }
  }

  Future<BridgeKey> createKey({
    required String cookies,
    required String label,
    bool outings = false,
  }) async {
    final body = await _post(
      '/v1/keys',
      cookies: cookies,
      body: {'label': label, 'outings': outings},
    );
    final key = BridgeKey(
      id: '${body['id'] ?? ''}',
      key: '${body['key'] ?? ''}',
      mcpUrl: '${body['mcpUrl'] ?? ''}',
    );
    if (key.id.isNotEmpty && key.key.isNotEmpty) await _keys.save(key);
    return key;
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
            phoneId: raw['phoneId'] is String ? raw['phoneId'] as String : null,
            outings: raw['outings'] == true,
          ),
        );
      }
    }
    if (body['thisPhone'] == true) {
      await _keys.retainOnly({for (final key in keys) key.id});
    }
    final phones = <BridgePhone>[];
    final rawPhones = body['phones'];
    if (rawPhones is List) {
      for (final raw in rawPhones) {
        if (raw is! Map) continue;
        final label = raw['label'];
        phones.add(
          BridgePhone(
            id: '${raw['id'] ?? ''}',
            label: label is String && label.isNotEmpty ? label : null,
            addedAt: _dateTimeFromUnixSeconds(raw['addedAt']),
            seenAt: _dateTimeFromUnixSeconds(raw['seenAt']),
            thisPhone: raw['thisPhone'] == true,
          ),
        );
      }
    }
    final phoneCount = body['phoneCount'];
    return BridgeAccount(
      registrationNumber: '${body['registrationNumber'] ?? ''}',
      linked: body['linked'] == true,
      savedCredentials: body['savedCredentials'] == true,
      phoneLinked: body['phoneLinked'] == true,
      thisPhone: body['thisPhone'] == true,
      keys: keys,
      phones: phones,
      phoneCount: phoneCount is int ? phoneCount : phones.length,
      settings: BridgeSettings.fromJson(body['settings']),
      semesterId: body['semesterId'] is String
          ? body['semesterId'] as String
          : null,
      vaultSavedAt: body['vaultSavedAt'] == null
          ? null
          : _dateTimeFromUnixSeconds(body['vaultSavedAt']),
      credentialsRejectedAt: body['credentialsRejectedAt'] == null
          ? null
          : _dateTimeFromUnixSeconds(body['credentialsRejectedAt']),
    );
  }

  /// Lets AI agents using key [id] apply for and cancel outings, or stops
  /// them.
  Future<void> setKeyOutings({
    required String cookies,
    required String id,
    required bool outings,
  }) async {
    await _post(
      '/v1/keys/outings',
      cookies: cookies,
      body: {'id': id, 'outings': outings},
    );
  }

  Future<void> revokeKey({required String cookies, required String id}) async {
    await _post('/v1/keys/revoke', cookies: cookies, body: {'id': id});
    await _keys.remove(id);
  }

  /// With [device], the account also moves to that install (after the app
  /// had to register a new one).
  Future<void> updateFcmToken({
    required String cookies,
    required String fcmToken,
    EdgeDevice? device,
  }) async {
    await _post(
      '/v1/account/fcm-token',
      cookies: cookies,
      body: {
        'fcmToken': fcmToken,
        ...?(device == null
            ? null
            : {'deviceId': device.id, 'deviceSecret': device.secret}),
      },
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

  Future<void> updateSemester({
    required String cookies,
    required String semesterId,
  }) async {
    await _post(
      '/v1/account/semester',
      cookies: cookies,
      body: {'semesterId': semesterId},
    );
  }

  /// Unlinks a phone; it is no longer asked for sessions. Unlinking this
  /// one also forgets its app secret.
  Future<void> removePhone({
    required String cookies,
    required BridgePhone phone,
  }) async {
    await _post(
      '/v1/account/remove-phone',
      cookies: cookies,
      body: {'id': phone.id},
    );
    if (phone.thisPhone) await _secrets.delete();
  }

  Future<void> deleteAccount({required String cookies}) async {
    await _post('/v1/account/delete', cookies: cookies);
    await _secrets.delete();
    await _keys.writeAll({});
  }
}
