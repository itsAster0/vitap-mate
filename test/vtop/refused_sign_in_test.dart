import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/vtop/client_provider.dart';

void main() {
  test('only a refused sign-in drops the saved session', () {
    expect(vtopRefusedSignIn(const VtopError.invalidCredentials()), isTrue);
    expect(
      vtopRefusedSignIn(const VtopError.authenticationFailed('no')),
      isTrue,
    );
    expect(vtopRefusedSignIn(const VtopError.sessionExpired()), isTrue);

    expect(vtopRefusedSignIn(const VtopError.vtopUnavailable('slow')), isFalse);
    expect(vtopRefusedSignIn(const VtopError.vtopServerError('500')), isFalse);
    expect(vtopRefusedSignIn(const VtopError.networkError()), isFalse);
    expect(
      vtopRefusedSignIn(VtopError.otpRequired('otp', BigInt.one)),
      isFalse,
    );
    expect(vtopRefusedSignIn(Exception('other')), isFalse);
  });
}
