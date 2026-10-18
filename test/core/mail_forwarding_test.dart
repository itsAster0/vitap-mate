import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/utils/edge/edge_device.dart';
import 'package:vitapmate/core/utils/edge/edge_events.dart';
import 'package:vitapmate/core/utils/email_otp/mail_forwarding_service.dart';

class _MemoryStore implements MailForwardingStore {
  MailForwarding? value;

  @override
  Future<MailForwarding?> read() async => value;

  @override
  Future<void> write(MailForwarding forwarding) async => value = forwarding;

  @override
  Future<void> delete() async => value = null;
}

class _MemoryDevices implements EdgeDeviceStore {
  EdgeDevice? value;

  @override
  Future<EdgeDevice?> read() async => value;

  @override
  Future<void> write(EdgeDevice device) async => value = device;
}

const _device = EdgeDevice(
  id: '0123456789abcdef0123456789abcdef',
  secret: 'vdv_secret',
  fcmToken: 'old',
);

const _forwarding = MailForwarding(
  id: 'k3f9x2ab7q',
  address: 'vtop-otp-k3f9x2ab7q@aster0.dev',
  secret: 'vmb_secret',
  privateKey: 'key',
  fcmToken: 'old',
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues({}));

  group('MailOtpPayload', () {
    test('reads an OTP box', () {
      final payload = MailOtpPayload.fromJson(
        '{"kind":"otp","code":"401295","sentAt":1760000000}',
      )!;
      expect(payload.isOtp, isTrue);
      expect(payload.code, '401295');
      expect(
        payload.sentAt,
        DateTime.fromMillisecondsSinceEpoch(1760000000000, isUtc: true),
      );
    });

    test('only takes OTPs sent for this challenge', () {
      final payload = MailOtpPayload.fromJson(
        '{"kind":"otp","code":"401295","sentAt":1760000000}',
      )!;
      final sent = payload.sentAt;
      expect(payload.isOtpFor(sent), isTrue);
      // Clock slack of a minute.
      expect(payload.isOtpFor(sent.add(const Duration(seconds: 50))), isTrue);
      expect(payload.isOtpFor(sent.add(const Duration(minutes: 5))), isFalse);
    });

    test('reads a Gmail confirmation', () {
      final payload = MailOtpPayload.fromJson(
        '{"kind":"gmail_confirm","code":"987654321","link":"https://x","requester":"a@gmail.com","sentAt":1}',
      )!;
      expect(payload.isGmailConfirmation, isTrue);
      expect(payload.isOtp, isFalse);
      expect(payload.requester, 'a@gmail.com');
    });

    test('rejects junk', () {
      expect(MailOtpPayload.fromJson('not json'), isNull);
      expect(MailOtpPayload.fromJson('{"code":"1"}'), isNull);
    });
  });

  group('EdgeEvent', () {
    test('reads socket events and FCM data alike', () {
      final socket = EdgeEvent.fromJson({
        'seq': 4,
        'kind': 'mail_otp',
        'mailbox': 'k3f9x2ab7q',
      })!;
      expect((socket.seq, socket.kind), (4, 'mail_otp'));
      final push = EdgeEvent.fromJson({'seq': '4', 'type': 'mail_otp'})!;
      expect((push.seq, push.kind), (4, 'mail_otp'));
      expect(EdgeEvent.fromJson({'type': 'mail_otp'}), isNull);
    });

    test('each sequence number is handled once', () async {
      final handled = <int>[];
      EdgeEventChannel.instance.on(
        'test_kind',
        (e) async => handled.add(e.seq),
      );
      final event = EdgeEvent(seq: 9001, kind: 'test_kind', data: const {});
      await EdgeEventChannel.instance.accept(event);
      await EdgeEventChannel.instance.accept(event);
      expect(handled, [9001]);
    });
  });

  group('MailForwardingService', () {
    test('syncs a changed FCM token to the install only', () async {
      final devices = _MemoryDevices()..value = _device;
      final calls = <(String, Map<String, dynamic>)>[];
      final client = MockClient((request) async {
        calls.add((
          request.url.path,
          jsonDecode(request.body) as Map<String, dynamic>,
        ));
        return http.Response('', 204);
      });
      final service = MailForwardingService(
        client: client,
        baseUrl: 'https://edge.example',
        store: _MemoryStore()..value = _forwarding,
        devices: EdgeDeviceService(
          client: client,
          baseUrl: 'https://edge.example',
          store: devices,
        ),
      );
      await service.syncFcmToken('old');
      expect(calls, isEmpty);
      await service.syncFcmToken('new');
      expect(calls.single.$1, '/v1/devices/update');
      expect(calls.single.$2, {
        'id': _device.id,
        'secret': 'vdv_secret',
        'fcmToken': 'new',
      });
      expect(devices.value!.fcmToken, 'new');
    });

    test(
      'an install the edge lost registers again and the address follows',
      () async {
        final devices = _MemoryDevices()..value = _device;
        final paths = <String>[];
        final client = MockClient((request) async {
          paths.add(request.url.path);
          return switch (request.url.path) {
            '/v1/devices/update' => http.Response(
              '{"code":"unknown_device","error":"gone"}',
              404,
            ),
            '/v1/devices/register' => http.Response(
              '{"id":"fedcba9876543210fedcba9876543210","secret":"vdv_new"}',
              201,
            ),
            _ => http.Response('', 204),
          };
        });
        final service = MailForwardingService(
          client: client,
          baseUrl: 'https://edge.example',
          store: _MemoryStore()..value = _forwarding,
          devices: EdgeDeviceService(
            client: client,
            baseUrl: 'https://edge.example',
            store: devices,
          ),
        );
        await service.syncFcmToken('new');
        expect(paths, [
          '/v1/devices/update',
          '/v1/devices/register',
          '/v1/mail-otp/update',
        ]);
        expect(devices.value!.secret, 'vdv_new');
      },
    );

    test('remove forgets the address even when the bridge lost it', () async {
      final store = _MemoryStore()..value = _forwarding;
      final service = MailForwardingService(
        client: MockClient(
          (_) async =>
              http.Response('{"code":"mailbox_not_found","error":"gone"}', 404),
        ),
        baseUrl: 'https://edge.example',
        store: store,
      );
      await service.remove();
      expect(store.value, isNull);
    });

    test('an empty inbox is null', () async {
      final service = MailForwardingService(
        client: MockClient((_) async => http.Response('{"sealed":null}', 200)),
        baseUrl: 'https://edge.example',
        store: _MemoryStore()..value = _forwarding,
      );
      expect(await service.pullInbox(), isNull);
    });

    test('pushes for another mailbox are ignored', () async {
      final service = MailForwardingService(
        client: MockClient((_) async => fail('no HTTP expected')),
        store: _MemoryStore()..value = _forwarding,
      );
      expect(
        await service.handlePush({'mailbox': 'other12345', 'sealed': 'AQ'}),
        isNull,
      );
    });
  });
}
