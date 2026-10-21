import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/providers/vtop_proxy.dart';

void main() {
  test('the phone reaches VTOP through the server only when asked', () {
    expect(
      vtopBaseUrlFor(proxy: true, onDevice: false, server: 'https://s/'),
      'https://s',
    );
    expect(
      vtopBaseUrlFor(proxy: false, onDevice: false, server: 'https://s'),
      null,
    );
  });

  test('on-device mode never uses the server, proxy or not', () {
    expect(
      vtopBaseUrlFor(proxy: true, onDevice: true, server: 'https://s'),
      null,
    );
  });
}
