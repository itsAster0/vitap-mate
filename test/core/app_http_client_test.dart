import 'package:flutter_test/flutter_test.dart';
import 'package:http/io_client.dart';
import 'package:vitapmate/core/utils/app_http_client.dart';

void main() {
  test('uses IOClient off Android and iOS', () {
    final client = createAppHttpClient();
    addTearDown(client.close);
    expect(client, isA<IOClient>());
  });
}
