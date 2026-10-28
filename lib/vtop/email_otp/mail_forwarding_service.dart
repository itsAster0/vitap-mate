import 'dart:async';
import 'dart:convert';

import 'package:vitapmate/observability/log.dart';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/foundation.dart'
    show defaultTargetPlatform, kIsWeb, TargetPlatform;
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/vtop/route/server_url.dart';
import 'package:vitapmate/vtop/edge/edge_device.dart';
import 'package:vitapmate/vtop/edge/edge_events.dart';
import 'package:vitapmate/src/api/mail_otp.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';

part 'mail_forwarding_service.g.dart';

/// FCM data `type` of a forwarded OTP box.
const mailOtpMessageType = 'mail_otp';

/// A forwarding address (`vtop-otp-<id>@<domain>`). The private key and
/// the mailbox secret stay in secure storage; the edge only has the public
/// key, so it relays boxes it cannot open.
class MailForwarding {
  const MailForwarding({
    required this.id,
    required this.address,
    required this.secret,
    required this.privateKey,
    this.fcmToken,
    this.lastOtpAt,
    this.ownerClaimed = false,
  });

  final String id;
  final String address;

  /// Proves to the bridge that this phone owns the mailbox.
  final String secret;
  final String privateKey;

  /// The FCM token the bridge last got.
  final String? fcmToken;

  /// When a forwarded OTP last arrived: proof the Gmail filter works.
  final DateTime? lastOtpAt;

  /// The bridge knows whose address this is, so a new phone signed in to
  /// the same VTOP account can take it back.
  final bool ownerClaimed;

  MailForwarding copyWith({
    String? fcmToken,
    DateTime? lastOtpAt,
    bool? ownerClaimed,
  }) => MailForwarding(
    id: id,
    address: address,
    secret: secret,
    privateKey: privateKey,
    fcmToken: fcmToken ?? this.fcmToken,
    lastOtpAt: lastOtpAt ?? this.lastOtpAt,
    ownerClaimed: ownerClaimed ?? this.ownerClaimed,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'address': address,
    'secret': secret,
    'privateKey': privateKey,
    'fcmToken': fcmToken,
    'lastOtpAt': lastOtpAt?.millisecondsSinceEpoch,
    'ownerClaimed': ownerClaimed,
  };

  static MailForwarding? fromJson(Object? json) {
    if (json is! Map) return null;
    final id = json['id'], address = json['address'];
    final secret = json['secret'], privateKey = json['privateKey'];
    if (id is! String || address is! String) return null;
    if (secret is! String || privateKey is! String) return null;
    final fcmToken = json['fcmToken'];
    final lastOtpAt = json['lastOtpAt'];
    return MailForwarding(
      id: id,
      address: address,
      secret: secret,
      privateKey: privateKey,
      fcmToken: fcmToken is String ? fcmToken : null,
      lastOtpAt: lastOtpAt is int
          ? DateTime.fromMillisecondsSinceEpoch(lastOtpAt)
          : null,
      ownerClaimed: json['ownerClaimed'] == true,
    );
  }
}

/// One install an address forwards to, as the edge lists them.
class MailForwardingDevice {
  const MailForwardingDevice({
    required this.deviceId,
    required this.label,
    required this.addedAt,
    required this.seenAt,
    required this.thisDevice,
  });

  final String deviceId;

  /// The name the install gave itself ("OnePlus CPH2573"), if any.
  final String? label;
  final DateTime addedAt;
  final DateTime seenAt;
  final bool thisDevice;

  static MailForwardingDevice? fromJson(Object? json) {
    if (json is! Map) return null;
    final deviceId = json['deviceId'], label = json['label'];
    final addedAt = json['addedAt'], seenAt = json['seenAt'];
    if (deviceId is! String || addedAt is! int) return null;
    DateTime at(int seconds) =>
        DateTime.fromMillisecondsSinceEpoch(seconds * 1000);
    return MailForwardingDevice(
      deviceId: deviceId,
      label: label is String && label.isNotEmpty ? label : null,
      addedAt: at(addedAt),
      seenAt: at(seenAt is int && seenAt > 0 ? seenAt : addedAt),
      thisDevice: json['thisDevice'] == true,
    );
  }
}

/// This install's name in the device list: maker and model, or null.
Future<String?> currentDeviceLabel() async {
  if (kIsWeb || defaultTargetPlatform != TargetPlatform.android) return null;
  try {
    final info = await DeviceInfoPlugin().androidInfo;
    final brand = info.brand.isEmpty
        ? ''
        : '${info.brand[0].toUpperCase()}${info.brand.substring(1)} ';
    final name = '$brand${info.model}'.trim();
    if (name.isEmpty) return null;
    return info.isPhysicalDevice ? name : 'Emulator ($name)';
  } catch (_) {
    return null;
  }
}

/// What a forwarded mail carried, after opening the box on this phone.
class MailOtpPayload {
  const MailOtpPayload({
    required this.kind,
    required this.sentAt,
    this.code,
    this.link,
    this.requester,
  });

  /// `otp` (VTOP's code) or `gmail_confirm` (Gmail's forwarding check).
  final String kind;
  final DateTime sentAt;
  final String? code;
  final String? link;

  /// The Gmail address asking to forward, for `gmail_confirm`.
  final String? requester;

  bool get isOtp => kind == 'otp' && (code?.length ?? 0) == 6;
  bool get isGmailConfirmation => kind == 'gmail_confirm';

  /// An OTP sent for a challenge that started at [challengeStart]. VTOP's
  /// clock and the mail's `Date` may differ a little.
  bool isOtpFor(DateTime challengeStart) =>
      isOtp &&
      !sentAt.isBefore(challengeStart.subtract(const Duration(seconds: 60)));

  static MailOtpPayload? fromJson(String text) {
    try {
      final json = jsonDecode(text);
      if (json is! Map) return null;
      final kind = json['kind'];
      if (kind is! String) return null;
      final sentAt = json['sentAt'];
      return MailOtpPayload(
        kind: kind,
        sentAt: DateTime.fromMillisecondsSinceEpoch(
          (sentAt is int ? sentAt : 0) * 1000,
          isUtc: true,
        ),
        code: json['code'] is String ? json['code'] as String : null,
        link: json['link'] is String ? json['link'] as String : null,
        requester: json['requester'] is String
            ? json['requester'] as String
            : null,
      );
    } catch (_) {
      return null;
    }
  }
}

class MailForwardingException implements Exception, UnknownDeviceError {
  const MailForwardingException(this.code, this.message, {this.statusCode});

  final String code;
  final String message;
  final int? statusCode;

  /// The bridge no longer knows this mailbox (or the secret is wrong).
  bool get mailboxGone => code == 'mailbox_not_found';

  /// The student already has another address; join that one instead.
  bool get addressExists => code == 'address_exists';

  /// The edge does not know this install; see [isUnknownDevice].
  @override
  bool get unknownDevice => isUnknownDevice(code, message, statusCode);

  @override
  String toString() => message;
}

abstract class MailForwardingStore {
  Future<MailForwarding?> read();
  Future<void> write(MailForwarding forwarding);
  Future<void> delete();
}

class SecureMailForwardingStore implements MailForwardingStore {
  const SecureMailForwardingStore();

  static const _key = 'mail_otp_forwarding_v1';
  static const _storage = FlutterSecureStorage();

  @override
  Future<MailForwarding?> read() async {
    final raw = await _storage.read(key: _key);
    if (raw == null) return null;
    try {
      return MailForwarding.fromJson(jsonDecode(raw));
    } catch (_) {
      return null;
    }
  }

  @override
  Future<void> write(MailForwarding forwarding) =>
      _storage.write(key: _key, value: jsonEncode(forwarding.toJson()));

  @override
  Future<void> delete() => _storage.delete(key: _key);
}

final _events = StreamController<MailOtpPayload>.broadcast();

/// Boxes opened from FCM pushes and inbox pulls, in this isolate.
Stream<MailOtpPayload> get mailOtpEvents => _events.stream;

/// An edge event carrying a box: opened like a push.
Future<void> handleMailOtpEvent(EdgeEvent event) async {
  await MailForwardingService().handlePush(event.data);
}

/// Talks to the edge's `/v1/mail-otp` routes and opens boxes locally.
/// Never logs codes, secrets or keys.
class MailForwardingService {
  MailForwardingService({
    http.Client? client,
    String? baseUrl,
    MailForwardingStore store = const SecureMailForwardingStore(),
    EdgeDeviceService? devices,
    Future<String?> Function() deviceLabel = currentDeviceLabel,
    MailOtpKeypair Function() keypair = mailOtpGenerateKeypair,
  }) : _client = client ?? sharedAppHttpClient,
       _baseUrl = baseUrl,
       _store = store,
       _devices =
           devices ?? EdgeDeviceService(client: client, baseUrl: baseUrl),
       _deviceLabel = deviceLabel,
       _keypair = keypair;

  final http.Client _client;

  /// Fixed for tests; otherwise the server URL in use at each call.
  final String? _baseUrl;
  final MailForwardingStore _store;
  final EdgeDeviceService _devices;
  final Future<String?> Function() _deviceLabel;
  final MailOtpKeypair Function() _keypair;

  Future<MailForwarding?> load() => _store.read();

  Future<bool> isReady() async => await _store.read() != null;

  Future<Map<String, dynamic>> _post(
    String path,
    Map<String, dynamic> body,
  ) async {
    try {
      final base = _baseUrl ?? await currentServerUrl();
      return await edgePost(_client, base, '/v1/mail-otp/$path', body);
    } on EdgeException catch (error) {
      throw MailForwardingException(
        error.code,
        error.message,
        statusCode: error.statusCode,
      );
    }
  }

  /// Runs [call] with this install, registering it again when the edge
  /// lost it (see [EdgeDeviceService.withInstall]).
  Future<T> _withDevice<T>(
    String fcmToken,
    Future<T> Function(EdgeDevice device) call,
  ) async {
    try {
      return await _devices.withInstall(fcmToken, call);
    } on EdgeException catch (error) {
      throw MailForwardingException(
        error.code,
        error.message,
        statusCode: error.statusCode,
      );
    }
  }

  /// Creates a new forwarding address with a fresh keypair, replacing any
  /// previous one on this phone.
  Future<MailForwarding> register({required String fcmToken}) async {
    final keypair = _keypair();
    final label = await _deviceLabel();
    final body = await _withDevice(
      fcmToken,
      (device) => _post('register', {
        'publicKey': keypair.publicKey,
        'deviceId': device.id,
        'deviceSecret': device.secret,
        'label': ?label,
      }),
    );
    final id = body['id'], address = body['address'], secret = body['secret'];
    if (id is! String || address is! String || secret is! String) {
      throw const MailForwardingException(
        'bad_response',
        'The bridge sent an unexpected answer.',
      );
    }
    final previous = await _store.read();
    final forwarding = MailForwarding(
      id: id,
      address: address,
      secret: secret,
      privateKey: keypair.secret,
      fcmToken: fcmToken,
    );
    await _store.write(forwarding);
    if (previous != null) unawaited(_deleteRemote(previous));
    unawaited(EdgeEventChannel.instance.connect());
    return forwarding;
  }

  /// Ties this phone's address to the student signed in with [cookies]
  /// (the edge has the bridge ask VTOP whose session it is and keeps only a
  /// keyed hash), so [reclaim] works on their next phone.
  Future<void> claimOwner({required String cookies}) async {
    final forwarding = await _store.read();
    if (forwarding == null) return;
    await _post('owner', {
      'id': forwarding.id,
      'secret': forwarding.secret,
      'session': {'cookies': cookies},
    });
    await _store.write(forwarding.copyWith(ownerClaimed: true));
  }

  /// Adds this install to the address of the student signed in with
  /// [cookies], with its own key pair; their other devices keep getting
  /// OTPs too. Null when they have no address on the edge.
  Future<MailForwarding?> reclaim({
    required String cookies,
    required String fcmToken,
  }) async {
    final keypair = _keypair();
    final label = await _deviceLabel();
    final Map<String, dynamic> body;
    try {
      body = await _withDevice(
        fcmToken,
        (device) => _post('reclaim', {
          'session': {'cookies': cookies},
          'publicKey': keypair.publicKey,
          'deviceId': device.id,
          'deviceSecret': device.secret,
          'label': ?label,
        }),
      );
    } on MailForwardingException catch (error) {
      if (error.mailboxGone) return null;
      rethrow;
    }
    final id = body['id'], address = body['address'], secret = body['secret'];
    if (id is! String || address is! String || secret is! String) {
      throw const MailForwardingException(
        'bad_response',
        'The bridge sent an unexpected answer.',
      );
    }
    final forwarding = MailForwarding(
      id: id,
      address: address,
      secret: secret,
      privateKey: keypair.secret,
      fcmToken: fcmToken,
      ownerClaimed: true,
    );
    await _store.write(forwarding);
    unawaited(EdgeEventChannel.instance.connect());
    return forwarding;
  }

  /// Tells the edge about a new FCM token, when it changed. Pushes go to
  /// the install, so the address follows it if it had to register again.
  Future<void> syncFcmToken(String? token) async {
    final EdgeDevice? moved;
    try {
      moved = await _devices.syncFcmToken(token);
    } on EdgeException catch (error) {
      throw MailForwardingException(
        error.code,
        error.message,
        statusCode: error.statusCode,
      );
    }
    if (moved == null) return;
    final forwarding = await _store.read();
    if (forwarding != null) {
      await _post('update', {
        'id': forwarding.id,
        'secret': forwarding.secret,
        'deviceId': moved.id,
        'deviceSecret': moved.secret,
      });
      await _store.write(forwarding.copyWith(fcmToken: moved.fcmToken));
    }
    EdgeEventChannel.instance.restart();
  }

  /// Takes this install off the address and forgets it here. With
  /// [everywhere], or when no other device uses it, the address itself is
  /// deleted: mail forwarded to it is rejected from then on.
  Future<void> remove({bool everywhere = false}) async {
    final forwarding = await _store.read();
    if (forwarding == null) return;
    await _deleteRemote(forwarding, everywhere: everywhere);
    await _store.delete();
  }

  Future<void> _deleteRemote(
    MailForwarding forwarding, {
    bool everywhere = false,
  }) async {
    try {
      await _post('delete', {
        'id': forwarding.id,
        'secret': forwarding.secret,
        if (everywhere) 'everywhere': true,
      });
    } on MailForwardingException catch (error) {
      if (!error.mailboxGone) rethrow;
    }
  }

  /// Forgets this phone's copy without calling the edge: for an address
  /// the edge no longer lets this install use.
  Future<void> forgetLocal() => _store.delete();

  /// The devices this address forwards to, this one included.
  Future<List<MailForwardingDevice>> devices() async {
    final forwarding = await _store.read();
    if (forwarding == null) return const [];
    final body = await _post('devices', {
      'id': forwarding.id,
      'secret': forwarding.secret,
    });
    final list = body['devices'];
    if (list is! List) return const [];
    return [for (final item in list) ?MailForwardingDevice.fromJson(item)];
  }

  /// Takes another device off the address; it stops getting OTPs.
  Future<void> removeDevice(String deviceId) async {
    final forwarding = await _store.read();
    if (forwarding == null) return;
    await _post('devices/remove', {
      'id': forwarding.id,
      'secret': forwarding.secret,
      'removeDevice': deviceId,
    });
  }

  /// The latest box the edge holds (kept 5 minutes), opened. For when a
  /// push did not arrive. Also emitted on [mailOtpEvents].
  Future<MailOtpPayload?> pullInbox() async {
    final forwarding = await _store.read();
    if (forwarding == null) return null;
    final body = await _post('inbox', {
      'id': forwarding.id,
      'secret': forwarding.secret,
    });
    final sealed = body['sealed'];
    if (sealed is! String || sealed.isEmpty) return null;
    return _deliver(forwarding, sealed);
  }

  /// Opens a pushed box (FCM data) and emits it on [mailOtpEvents].
  Future<MailOtpPayload?> handlePush(Map<String, dynamic> data) async {
    final forwarding = await _store.read();
    final mailbox = '${data['mailbox'] ?? ''}';
    final sealed = '${data['sealed'] ?? ''}';
    if (forwarding == null || mailbox != forwarding.id || sealed.isEmpty) {
      Log.info('mail.otp', 'Ignoring a mail OTP push for another mailbox');
      return null;
    }
    return _deliver(forwarding, sealed);
  }

  Future<MailOtpPayload?> _deliver(
    MailForwarding forwarding,
    String sealed,
  ) async {
    final payload = _open(forwarding, sealed);
    if (payload == null) return null;
    if (payload.isOtp) {
      await _store.write(forwarding.copyWith(lastOtpAt: DateTime.now()));
    }
    _events.add(payload);
    return payload;
  }

  MailOtpPayload? _open(MailForwarding forwarding, String sealed) {
    try {
      return MailOtpPayload.fromJson(
        mailOtpOpen(
          secret: forwarding.privateKey,
          mailboxId: forwarding.id,
          sealed: sealed,
        ),
      );
    } catch (error) {
      Log.info('mail.otp', 'Could not open a mail OTP box: $error');
      return null;
    }
  }
}

@Riverpod(keepAlive: true)
MailForwardingService mailForwardingService(Ref ref) => MailForwardingService();

@riverpod
Future<MailForwarding?> mailForwarding(Ref ref) =>
    ref.watch(mailForwardingServiceProvider).load();
