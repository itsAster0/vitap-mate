import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/vtop_backend/vtop_server_settings.dart';
import 'package:vitapmate/features/settings/presentation/widgets/vtop_server_dialog.dart';

void main() {
  test('Data Source tile label follows the switch', () {
    const server = VtopServerSettings(
      url: 'https://vtop.example.com/',
      apiKey: '',
    );
    expect(vtopDataSourceLabel(server), 'Server · vtop.example.com');
    expect(
      vtopDataSourceLabel(server.copyWith(enabled: false)),
      'On device · server saved',
    );
    expect(vtopDataSourceLabel(VtopServerSettings.disabled), 'On device');
  });
}
