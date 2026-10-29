import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/vtop/route/vtop_udp.dart';

const _key = '12f05307ad08111877036e4312a80ce4327d814ea1fe821ca5ba0ef51e02e04c';

void main() {
  test("reads the server's answer", () {
    final gateway = udpGatewayFromJson({
      'host': 'gw.example',
      'ports': [3478, 51820, 8443],
      'publicKey': _key,
      'expiresAt': 1793563200,
    })!;
    expect(gateway.host, 'gw.example');
    expect(gateway.ports, [3478, 51820, 8443]);
    expect((gateway.publicKey, gateway.expiresAt), (_key, 1793563200));
  });

  test('refuses a broken answer', () {
    expect(udpGatewayFromJson({'host': 'gw.example'}), isNull);
    expect(
      udpGatewayFromJson({
        'host': 'gw.example',
        'ports': [3478],
        'publicKey': 'not hex',
        'expiresAt': 1,
      }),
      isNull,
    );
  });

  test('a key is 64 hex digits', () {
    expect(isUdpKey(_key), isTrue);
    expect(isUdpKey(_key.toUpperCase()), isFalse);
    expect(isUdpKey(_key.substring(1)), isFalse);
  });
}
