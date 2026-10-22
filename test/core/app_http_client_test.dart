import 'package:flutter_test/flutter_test.dart';
import 'package:http/io_client.dart';
import 'package:vitapmate/core/utils/app_http_client.dart';

void main() {
  test('uses IOClient off Android and iOS', () {
    final client = createAppHttpClient();
    addTearDown(client.close);
    expect(client, isA<IOClient>());
  });

  group('withoutBrokenAlternativeServices', () {
    test('drops saved QUIC failures and keeps the rest', () {
      final json = {
        'net': {
          'http_server_properties': {
            'broken_alternative_services': [
              {'host': 'vmate-api.aster0.dev', 'broken_count': 1},
            ],
            'servers': [
              {'server': 'https://vmate-api.aster0.dev', 'supports_quic': true},
            ],
            'version': 5,
          },
          'host_cache': {'entries': []},
        },
      };
      expect(withoutBrokenAlternativeServices(json), isTrue);
      final props =
          (json['net']! as Map)['http_server_properties']
              as Map<String, Object>;
      expect(props.keys, unorderedEquals(['servers', 'version']));
      expect((json['net']! as Map).containsKey('host_cache'), isTrue);
    });

    test('leaves prefs without failures, or of another shape, alone', () {
      expect(
        withoutBrokenAlternativeServices({
          'net': {
            'http_server_properties': {'servers': []},
          },
        }),
        isFalse,
      );
      expect(withoutBrokenAlternativeServices({'other': 1}), isFalse);
      expect(withoutBrokenAlternativeServices(null), isFalse);
    });
  });
}
