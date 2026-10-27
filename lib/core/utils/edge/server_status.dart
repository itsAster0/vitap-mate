import 'dart:convert';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/providers/server_url.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/app_http_client.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';

/// `GET /healthz`, with the round trip measured here.
class ServerHealth {
  const ServerHealth({
    required this.ok,
    required this.rttMs,
    this.version,
    this.colo,
  });

  final bool ok;
  final int rttMs;
  final String? version;

  /// The Cloudflare location that answered (`SIN`, `BOM`, …).
  final String? colo;
}

Future<ServerHealth> checkHealth(String url, http.Client client) async {
  final watch = Stopwatch()..start();
  try {
    final response = await client
        .get(Uri.parse('$url/healthz'))
        .timeout(const Duration(seconds: 8));
    final body = jsonDecode(response.body);
    final ok =
        response.statusCode == 200 &&
        body is Map &&
        body['service'] == 'vtop-edge';
    return ServerHealth(
      ok: ok,
      rttMs: watch.elapsedMilliseconds,
      version: ok && body['version'] is String
          ? body['version'] as String
          : null,
      colo: ok && body['colo'] is String ? body['colo'] as String : null,
    );
  } catch (_) {
    return ServerHealth(ok: false, rttMs: watch.elapsedMilliseconds);
  }
}

DateTime? _time(Object? seconds) => seconds is int
    ? DateTime.fromMillisecondsSinceEpoch(seconds * 1000, isUtc: true)
    : null;

class SignInEntry {
  const SignInEntry({
    required this.at,
    required this.byServer,
    required this.ok,
    this.reason,
  });

  final DateTime at;
  final bool byServer;
  final bool ok;
  final String? reason;
}

class KeyUse {
  const KeyUse({required this.at, required this.via});

  final DateTime at;

  /// `extension` or `mcp`.
  final String via;
}

class KeyActivity {
  const KeyActivity({
    required this.id,
    required this.label,
    required this.lastUsedAt,
    required this.uses,
  });

  final String id;
  final String label;
  final DateTime? lastUsedAt;
  final List<KeyUse> uses;
}

/// `/v1/app/status`: what the server keeps for this account and its
/// recent activity.
class ServerStatus {
  const ServerStatus({
    this.sessionSavedAt,
    this.sessionSource,
    this.vaultSavedAt,
    this.vaultExpiresAt,
    this.vaultGmail = false,
    this.credentialsRejectedAt,
    this.forwarding,
    this.phones = 0,
    this.signIns = const [],
    this.keys = const [],
  });

  final DateTime? sessionSavedAt;

  /// Who made the cached session: `phone` or `server`.
  final String? sessionSource;

  /// When the sealed password was saved, and when it will be forgotten.
  final DateTime? vaultSavedAt;
  final DateTime? vaultExpiresAt;

  /// The sealed password carries Gmail access, so the server reads the OTP
  /// from Gmail when there is no forwarding address.
  final bool vaultGmail;

  /// When VTOP last refused the saved password.
  final DateTime? credentialsRejectedAt;

  /// The OTP forwarding address, when there is one.
  final String? forwarding;

  /// Linked phones.
  final int phones;
  final List<SignInEntry> signIns;
  final List<KeyActivity> keys;

  factory ServerStatus.fromJson(Map<String, dynamic> json) {
    List<Map> maps(Object? value) => [
      if (value is List) ...value.whereType<Map>(),
    ];
    return ServerStatus(
      sessionSavedAt: _time(json['sessionSavedAt']),
      sessionSource: json['sessionSource'] as String?,
      vaultSavedAt: _time(json['vaultSavedAt']),
      vaultExpiresAt: _time(json['vaultExpiresAt']),
      vaultGmail: json['vaultGmail'] == true,
      credentialsRejectedAt: _time(json['credentialsRejectedAt']),
      forwarding: json['forwarding'] as String?,
      phones: json['phones'] is int ? json['phones'] as int : 0,
      signIns: [
        for (final s in maps(json['signIns']))
          if (_time(s['at']) case final at?)
            SignInEntry(
              at: at,
              byServer: s['by'] == 'server',
              ok: s['ok'] == true,
              reason: s['reason'] as String?,
            ),
      ],
      keys: [
        for (final k in maps(json['keys']))
          KeyActivity(
            id: '${k['id'] ?? ''}',
            label: '${k['label'] ?? ''}',
            lastUsedAt: _time(k['lastUsedAt']),
            uses: [
              for (final u in maps(k['uses']))
                if (_time(u['at']) case final at?)
                  KeyUse(at: at, via: '${u['via'] ?? 'extension'}'),
            ],
          ),
      ],
    );
  }
}

/// Who is asking: a phone linked to the account.
typedef EdgeIdentity = ({String appSecret, String registrationNumber});

/// The phone could not read its own app secret or registration number, so
/// it could not ask. Unlike [ServerNotLinked], it leaves the link alone.
class ServerIdentityUnavailable implements Exception {
  const ServerIdentityUnavailable();

  @override
  String toString() => 'This phone could not read its server link.';
}

/// The server no longer counts this phone as linked.
class ServerNotLinked implements Exception {
  const ServerNotLinked();

  @override
  String toString() => 'This phone is not linked to the server.';
}

class ServerStatusService {
  ServerStatusService({
    required http.Client client,
    required Future<EdgeIdentity?> Function() identity,
    Future<String> Function()? serverUrl,
  }) : _client = client,
       _identity = identity,
       _serverUrl = serverUrl ?? currentServerUrl;

  final http.Client _client;
  final Future<EdgeIdentity?> Function() _identity;
  final Future<String> Function() _serverUrl;

  Future<Map<String, dynamic>> _post(String path) async {
    final identity = await _identity();
    if (identity == null) throw const ServerIdentityUnavailable();
    final response = await _client
        .post(
          Uri.parse('${await _serverUrl()}$path'),
          headers: {'content-type': 'application/json'},
          body: jsonEncode({
            'appSecret': identity.appSecret,
            'registrationNumber': identity.registrationNumber,
          }),
        )
        .timeout(const Duration(seconds: 15));
    Object? body;
    try {
      body = jsonDecode(response.body);
    } catch (_) {}
    final json = body is Map<String, dynamic> ? body : <String, dynamic>{};
    if (response.statusCode == 403 && json['code'] == 'not_linked') {
      throw const ServerNotLinked();
    }
    if (response.statusCode != 200) {
      throw StateError(
        '${json['error'] ?? 'The server answered ${response.statusCode}.'}',
      );
    }
    return json;
  }

  Future<ServerStatus> status() async =>
      ServerStatus.fromJson(await _post('/v1/app/status'));

  Future<void> clearSession() => _post('/v1/app/session/clear');
}

final serverHealthProvider = FutureProvider.autoDispose<ServerHealth>(
  (ref) => checkHealth(
    ref.watch(serverUrlProvider),
    ref.watch(appHttpClientProvider),
  ),
);

final serverStatusServiceProvider = Provider<ServerStatusService>(
  (ref) => ServerStatusService(
    client: ref.watch(appHttpClientProvider),
    identity: () => edgeIdentity(ref),
  ),
);

/// The account as the server sees it. When the server says this phone is
/// no longer linked, the phone forgets it is (so the mode falls back to the
/// phone).
final serverStatusProvider = FutureProvider.autoDispose<ServerStatus>(
  retry: (count, error) => error is ServerNotLinked || count >= 3
      ? null
      : Duration(seconds: 2 << count),
  (ref) async {
    ref.watch(serverUrlProvider);
    try {
      return await ref.watch(serverStatusServiceProvider).status();
    } on ServerNotLinked {
      final prefs = await ref.read(settingsProvider.future);
      if (prefs.getBool(bridgeLinkedSettingKey) ?? false) {
        await prefs.setBool(bridgeLinkedSettingKey, false);
        ref.invalidate(settingsProvider);
      }
      rethrow;
    }
  },
);
