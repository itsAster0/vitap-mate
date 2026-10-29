import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';

void main() {
  test('the phone reaches VTOP through the server only when asked', () {
    for (final mode in [VtopServerMode.compress, VtopServerMode.server]) {
      expect(
        vtopBaseUrlFor(mode: mode, onDevice: false, server: 'https://s/'),
        'https://s',
      );
    }
    expect(
      vtopBaseUrlFor(
        mode: VtopServerMode.off,
        onDevice: false,
        server: 'https://s',
      ),
      null,
    );
  });

  test('on-device mode never uses the server, whatever the mode', () {
    for (final mode in VtopServerMode.values) {
      expect(
        vtopBaseUrlFor(mode: mode, onDevice: true, server: 'https://s'),
        null,
      );
    }
  });

  test('the old proxy switch carries over as Compress', () {
    expect(vtopModeFrom(oldProxy: true), VtopServerMode.compress);
    expect(vtopModeFrom(oldProxy: false), VtopServerMode.off);
    expect(
      vtopModeFrom(saved: 'server', oldProxy: true),
      VtopServerMode.server,
    );
    // Nothing chosen yet: compressed through the server.
    expect(vtopModeFrom(saved: 'nonsense'), VtopServerMode.compress);
    expect(vtopModeFrom(), VtopServerMode.compress);
  });
}
