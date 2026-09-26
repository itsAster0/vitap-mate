import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';

void main() {
  test('safe course reports how many classes can be skipped', () {
    const s = AttendanceStanding(attended: 22, total: 24);
    expect(s.isSafe, isTrue);
    // 22 / (24 + 5) = 75.9%, 22 / (24 + 6) = 73.3%
    expect(s.canSkip, 5);
    expect(s.advice, 'Can skip 5');
  });

  test('short course reports classes needed to recover', () {
    const s = AttendanceStanding(attended: 6, total: 10);
    expect(s.isSafe, isFalse);
    // (6 + 6) / (10 + 6) = 75%
    expect(s.mustAttend, 6);
    expect(s.advice, 'Attend 6');
  });

  test('exactly on the line cannot skip', () {
    const s = AttendanceStanding(attended: 3, total: 4);
    expect(s.canSkip, 0);
    expect(s.advice, "Don't skip");
  });

  test('no classes yet', () {
    const s = AttendanceStanding(attended: 0, total: 0);
    expect(s.isSafe, isTrue);
    expect(s.advice, 'No classes yet');
  });
}
