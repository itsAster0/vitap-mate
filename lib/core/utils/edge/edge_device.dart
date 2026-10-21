import 'dart:async';
import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/providers/on_device_mode.dart';
import 'package:vitapmate/core/providers/server_url.dart';
import 'package:vitapmate/core/utils/app_http_client.dart';

/// This install as the edge knows it. Events for it (forwarded OTPs, …)
/// go to its FCM token and, while the app is open, its WebSocket.
class EdgeDevice {
  const EdgeDevice({
    required this.id,
    required this.secret,
    required this.fcmToken,
  });

  final String id;

  /// Proves to the edge that this is the install; kept in secure storage.
  final String secret;

  /// The FCM token the edge last got.
  final String fcmToken;

  Map<String, dynamic> toJson() => {
    'id': id,
    'secret': secret,
    'fcmToken': fcmToken,
  };

  static EdgeDevice? fromJson(Object? json) {
    if (json is! Map) return null;
    final id = json['id'], secret = json['secret'], token = json['fcmToken'];
    if (id is! String || secret is! String || token is! String) return null;
    return EdgeDevice(id: id, secret: secret, fcmToken: token);
  }
}

/// The edge does not know this install (its data was reset, or the install
/// was registered with another server). Edges before `unknown_device` was
/// used everywhere answered 400 "Unknown device." on the mailbox and link
/// routes.
bool isUnknownDevice(String code, String message, int? statusCode) =>
    code == 'unknown_device' ||
    (statusCode == 400 && message.trim() == 'Unknown device.');

/// An error a call can fail with when the edge lost this install.
abstract interface class UnknownDeviceError {
  bool get unknownDevice;
}

class EdgeException implements Exception, UnknownDeviceError {
  const EdgeException(this.code, this.message, {this.statusCode});

  final String code;
  final String message;
  final int? statusCode;

  @override
  bool get unknownDevice => isUnknownDevice(code, message, statusCode);

  @override
  String toString() => message;
}

abstract class EdgeDeviceStore {
  Future<EdgeDevice?> read();
  Future<void> write(EdgeDevice device);
  Future<void> delete();
}

class SecureEdgeDeviceStore implements EdgeDeviceStore {
  const SecureEdgeDeviceStore();

  static const _key = 'edge_device_v1';
  static const _storage = FlutterSecureStorage();

  @override
  Future<EdgeDevice?> read() async {
    final raw = await _storage.read(key: _key);
    if (raw == null) return null;
    try {
      return EdgeDevice.fromJson(jsonDecode(raw));
    } catch (_) {
      return null;
    }
  }

  @override
  Future<void> write(EdgeDevice device) =>
      _storage.write(key: _key, value: jsonEncode(device.toJson()));

  @override
  Future<void> delete() => _storage.delete(key: _key);
}

/// Posts JSON to the edge; errors come back as [EdgeException].
Future<Map<String, dynamic>> edgePost(
  http.Client client,
  String baseUrl,
  String path,
  Map<String, dynamic> body,
) async {
  if (baseUrl.isEmpty) {
    throw const EdgeException('not_configured', 'No server URL is set.');
  }
  if (await isOnDeviceMode()) {
    throw const EdgeException(
      OnDeviceModeException.code,
      OnDeviceModeException.message,
    );
  }
  final response = await client
      .post(
        Uri.parse('$baseUrl$path'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode(body),
      )
      .timeout(const Duration(seconds: 15));
  Map<String, dynamic> decoded;
  try {
    final parsed = jsonDecode(response.body);
    decoded = parsed is Map<String, dynamic> ? parsed : const {};
  } catch (_) {
    decoded = const {};
  }
  final status = response.statusCode;
  if (status < 200 || status >= 300) {
    throw EdgeException(
      '${decoded['code'] ?? 'http_$status'}',
      '${decoded['error'] ?? 'The server answered HTTP $status.'}',
      statusCode: status,
    );
  }
  return decoded;
}

/// Registers this install with the edge and keeps its FCM token current.
class EdgeDeviceService {
  EdgeDeviceService({
    http.Client? client,
    String? baseUrl,
    EdgeDeviceStore store = const SecureEdgeDeviceStore(),
  }) : _client = client ?? sharedAppHttpClient,
       _baseUrl = baseUrl,
       _store = store;

  final http.Client _client;

  /// Fixed for tests; otherwise the server URL in use at each call.
  final String? _baseUrl;
  final EdgeDeviceStore _store;

  Future<String> _base() async => _baseUrl ?? await currentServerUrl();

  Future<EdgeDevice?> load() => _store.read();

  /// This install, registered first if it is not yet.
  Future<EdgeDevice> ensure(String fcmToken) async {
    final current = await _store.read();
    if (current == null) return _register(fcmToken);
    if (current.fcmToken == fcmToken) return current;
    await syncFcmToken(fcmToken);
    return await _store.read() ?? current;
  }

  /// Runs [call] with this install; when the edge no longer knows it, the
  /// install registers again and [call] runs once more with the new one.
  Future<T> withInstall<T>(
    String fcmToken,
    Future<T> Function(EdgeDevice device) call,
  ) async {
    try {
      return await call(await ensure(fcmToken));
    } on UnknownDeviceError catch (error) {
      if (!error.unknownDevice) rethrow;
    }
    return call(await _register(fcmToken));
  }

  Future<EdgeDevice> _register(String fcmToken) async {
    final body = await edgePost(
      _client,
      await _base(),
      '/v1/devices/register',
      {'fcmToken': fcmToken},
    );
    final id = body['id'], secret = body['secret'];
    if (id is! String || secret is! String) {
      throw const EdgeException(
        'bad_response',
        'The server sent an unexpected answer.',
      );
    }
    final device = EdgeDevice(id: id, secret: secret, fcmToken: fcmToken);
    await _store.write(device);
    return device;
  }

  /// Tells the edge about a new FCM token, when it changed. Returns the
  /// install when it had to be registered again (the edge lost it), so
  /// what pointed at the old one can move; null otherwise.
  Future<EdgeDevice?> syncFcmToken(String? token) async {
    if (token == null || token.isEmpty) return null;
    final current = await _store.read();
    if (current == null || current.fcmToken == token) return null;
    try {
      await edgePost(_client, await _base(), '/v1/devices/update', {
        'id': current.id,
        'secret': current.secret,
        'fcmToken': token,
      });
    } on EdgeException catch (error) {
      if (!error.unknownDevice) rethrow;
      return _register(token);
    }
    await _store.write(
      EdgeDevice(id: current.id, secret: current.secret, fcmToken: token),
    );
    return null;
  }
}
