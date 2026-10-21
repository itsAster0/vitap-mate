import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/utils/vtop_client_call.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

void main() {
  group('vtopErrorMessage', () {
    test('every VtopError says something specific, never the generic line', () {
      final messages = {
        for (final e in <VtopError>[
          const VtopError.networkError(),
          const VtopError.vtopServerError(''),
          const VtopError.authenticationFailed(''),
          const VtopError.registrationParsingError(),
          const VtopError.invalidCredentials(),
          const VtopError.sessionExpired(),
          const VtopError.parseError('CSRF token not found'),
          const VtopError.configurationError('bad semester'),
          const VtopError.captchaRequired(),
          const VtopError.invalidResponse(),
        ])
          e: vtopErrorMessage(e),
      };
      for (final entry in messages.entries) {
        expect(entry.value, isNotEmpty, reason: '${entry.key}');
        expect(
          entry.value,
          isNot(contains('Something went wrong')),
          reason: '${entry.key} should be specific',
        );
      }
    });

    test('a parse failure reads plainly and keeps the detail for context', () {
      final msg = vtopErrorMessage(
        const VtopError.parseError('CSRF token not found'),
      );
      expect(msg.toLowerCase(), contains("couldn't be read"));
      expect(msg, contains('CSRF token not found'));
      expect(msg.toLowerCase(), contains('try again'));
    });

    test('bad credentials point the student at Settings', () {
      expect(
        vtopErrorMessage(const VtopError.invalidCredentials()).toLowerCase(),
        contains('settings'),
      );
    });

    test('a non-VtopError is handled too', () {
      expect(vtopErrorMessage(Exception('x')), isNotEmpty);
    });
  });
}
