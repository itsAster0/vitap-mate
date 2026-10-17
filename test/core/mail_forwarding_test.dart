import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
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

const _forwarding = MailForwarding(
  id: 'k3f9x2ab7q',
  address: 'vtop-otp-k3f9x2ab7q@aster0.dev',
  secret: 'vmb_secret',
  privateKey: 'key',
  fcmToken: 'old',
);

void main() {
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

  group('MailForwardingService', () {
    test('syncs a changed FCM token only', () async {
      final store = _MemoryStore()..value = _forwarding;
      final calls = <Map<String, dynamic>>[];
      final service = MailForwardingService(
        client: MockClient((request) async {
          expect(request.url.path, '/v1/mail-otp/update');
          calls.add(jsonDecode(request.body) as Map<String, dynamic>);
          return http.Response('', 204);
        }),
        baseUrl: 'https://bridge.example',
        store: store,
      );
      await service.syncFcmToken('old');
      expect(calls, isEmpty);
      await service.syncFcmToken('new');
      expect(calls.single, {
        'id': 'k3f9x2ab7q',
        'secret': 'vmb_secret',
        'fcmToken': 'new',
      });
      expect(store.value!.fcmToken, 'new');
    });

    test('remove forgets the address even when the bridge lost it', () async {
      final store = _MemoryStore()..value = _forwarding;
      final service = MailForwardingService(
        client: MockClient(
          (_) async =>
              http.Response('{"code":"mailbox_not_found","error":"gone"}', 404),
        ),
        baseUrl: 'https://bridge.example',
        store: store,
      );
      await service.remove();
      expect(store.value, isNull);
    });

    test('an empty inbox is null', () async {
      final service = MailForwardingService(
        client: MockClient((_) async => http.Response('{"sealed":null}', 200)),
        baseUrl: 'https://bridge.example',
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
