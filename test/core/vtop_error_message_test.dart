import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/vtop/vtop_client_call.dart';
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
          const VtopError.vtopUnavailable('VTOP is down right now'),
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

    test('VTOP being down is blamed on VTOP, not the connection', () {
      const error = VtopError.vtopUnavailable('VTOP took too long to answer');
      final msg = vtopErrorMessage(error);
      expect(isVtopDown(error), isTrue);
      expect(msg, startsWith('VTOP took too long to answer.'));
      expect(msg, contains("VTOP's side, not your connection"));
      expect(msg.toLowerCase(), isNot(contains('check your internet')));
    });

    test(
      'a network error with no sign of being online stays a network error',
      () {
        const error = VtopError.networkError();
        expect(isVtopDown(error), isFalse);
        expect(
          vtopErrorMessage(error),
          contains('Check your internet connection'),
        );
      },
    );

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
