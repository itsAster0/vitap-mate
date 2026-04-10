import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/utils/registration_number.dart';

void main() {
  test('deriveRegistrationNumber extracts a VITAP registration number', () {
    expect(
      deriveRegistrationNumber('student.22bce1234@vitapstudent.ac.in'),
      '22bce1234',
    );
  });
}
