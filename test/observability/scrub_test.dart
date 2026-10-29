import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/observability/log.dart';

void main() {
  test('blanks secrets, keeps the rest', () {
    expect(scrub('login password=hunter2 ok'), 'login password=… ok');
    expect(scrub('OTP: 123456 sent'), 'OTP: … sent');
    expect(scrub('{"_csrf":"abc-1"}'), '{"_csrf":"…"}');
    expect(
      scrub('Cookie: JSESSIONID=XYZ; SERVERID=s1'),
      'Cookie: …; SERVERID=…',
    );
    expect(scrub('token expired'), 'token expired');
    expect(scrub('promptForOtp=true'), 'promptForOtp=true');
  });
}
