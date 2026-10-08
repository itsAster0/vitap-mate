import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/utils/bridge_otp_prompt.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

void main() {
  PendingBridgeOtp pending({int createdAtMs = 1000}) => PendingBridgeOtp(
    requestId: 'r1',
    responseToken: 'tok',
    callbackUrl: 'https://bridge.test/cookie/callback',
    wantCredentials: true,
    state: SessionState(
      cookies: 'JSESSIONID=a',
      csrfToken: 'csrf',
      registrationNumber: '24MIC7076',
      otpIssuedAt: BigInt.from(1790000000),
    ),
    createdAtMs: createdAtMs,
  );

  test('pending OTP round-trips through JSON', () {
    final restored = PendingBridgeOtp.fromJson(
      jsonDecode(jsonEncode(pending().toJson())) as Map<String, dynamic>,
    );
    expect(restored, isNotNull);
    expect(restored!.requestId, 'r1');
    expect(restored.responseToken, 'tok');
    expect(restored.callbackUrl, 'https://bridge.test/cookie/callback');
    expect(restored.wantCredentials, isTrue);
    expect(restored.state, pending().state);
  });

  test('malformed JSON is null', () {
    expect(PendingBridgeOtp.fromJson(const {'requestId': 'r1'}), isNull);
  });

  test('a pending OTP goes stale after ten minutes', () {
    final p = pending(createdAtMs: 0);
    expect(p.isStaleAt(DateTime.fromMillisecondsSinceEpoch(9 * 60 * 1000)), isFalse);
    expect(p.isStaleAt(DateTime.fromMillisecondsSinceEpoch(10 * 60 * 1000 + 1)), isTrue);
  });

  test('notification responses are classified', () {
    final payload = jsonEncode({'type': bridgeOtpPayloadType});
    expect(
      classifyBridgeOtpResponse(payload: payload, actionId: bridgeOtpReplyActionId, input: ' 123456 '),
      const BridgeOtpResponse.reply('123456'),
    );
    expect(
      classifyBridgeOtpResponse(payload: payload, actionId: null, input: null),
      const BridgeOtpResponse.tap(),
    );
    expect(
      classifyBridgeOtpResponse(payload: payload, actionId: bridgeOtpReplyActionId, input: 'abc'),
      const BridgeOtpResponse.invalidReply(),
    );
    expect(
      classifyBridgeOtpResponse(payload: '{"type":"class_reminder"}', actionId: null, input: null),
      isNull,
    );
    expect(classifyBridgeOtpResponse(payload: null, actionId: null, input: null), isNull);
  });
}
