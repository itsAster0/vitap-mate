import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/more/domain/gpa_calculator.dart';

void main() {
  group('GPA calculator', () {
    test('calculates a credit-weighted semester GPA', () {
      final courses = [
        GpaCourse(credits: Credits(4), grade: Grade.s),
        GpaCourse(credits: Credits(3), grade: Grade.a),
        GpaCourse(credits: Credits(2), grade: Grade.b),
      ];

      expect(calculateSemesterGpa(courses), closeTo(9.2222, 0.0001));
    });

    test('projects CGPA using completed and planned credits', () {
      final result = calculateProjectedCgpa(
        currentCgpa: Cgpa(8),
        completedCredits: 60,
        plannedCourses: [GpaCourse(credits: Credits(20), grade: Grade.s)],
      );

      expect(result, 8.5);
    });

    test('supports decimal credit values from grade history', () {
      expect(parseCredits('2.5')?.value, 2.5);
      expect(parseCredits('Credits: 3.0')?.value, 3);
      expect(parseCredits('not available'), isNull);
    });

    test('failed grades contribute zero points but retain credits', () {
      final courses = [
        GpaCourse(credits: Credits(3), grade: Grade.a),
        GpaCourse(credits: Credits(3), grade: Grade.f),
      ];

      expect(calculateSemesterGpa(courses), 4.5);
    });

    test('combines theory and lab credits without counting repeated slots', () {
      final courses = combineSemesterCourseCredits([
        SemesterCourseComponent(
          courseCode: 'CSE4007',
          courseType: 'ETH',
          credits: Credits(3),
        ),
        SemesterCourseComponent(
          courseCode: 'CSE4007',
          courseType: 'ETH',
          credits: Credits(3),
        ),
        SemesterCourseComponent(
          courseCode: 'CSE4007',
          courseType: 'ELA',
          credits: Credits(1),
        ),
      ]);

      expect(courses, hasLength(1));
      expect(courses.single.courseCode, 'CSE4007');
      expect(courses.single.credits.value, 4);
    });

    test('rejects invalid domain values before calculation', () {
      expect(() => Credits(0), throwsArgumentError);
      expect(() => Credits(-1), throwsArgumentError);
      expect(() => Cgpa(10.1), throwsArgumentError);
      expect(Grade.tryParse('unknown'), isNull);
    });
  });
}
