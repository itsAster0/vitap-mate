import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/more/domain/gpa_calculator.dart';

void main() {
  group('GPA calculator', () {
    test('calculates a credit-weighted semester GPA', () {
      const courses = [
        GpaCourse(credits: 4, grade: 'S'),
        GpaCourse(credits: 3, grade: 'A'),
        GpaCourse(credits: 2, grade: 'B'),
      ];

      expect(calculateSemesterGpa(courses), closeTo(9.2222, 0.0001));
    });

    test('projects CGPA using completed and planned credits', () {
      final result = calculateProjectedCgpa(
        currentCgpa: 8,
        completedCredits: 60,
        plannedCourses: const [GpaCourse(credits: 20, grade: 'S')],
      );

      expect(result, 8.5);
    });

    test('supports decimal credit values from grade history', () {
      expect(parseCredits('2.5'), 2.5);
      expect(parseCredits('Credits: 3.0'), 3);
      expect(parseCredits('not available'), isNull);
    });

    test('failed grades contribute zero points but retain credits', () {
      const courses = [
        GpaCourse(credits: 3, grade: 'A'),
        GpaCourse(credits: 3, grade: 'F'),
      ];

      expect(calculateSemesterGpa(courses), 4.5);
    });

    test('combines theory and lab credits without counting repeated slots', () {
      final courses = combineSemesterCourseCredits(const [
        SemesterCourseComponent(
          courseCode: 'CSE4007',
          courseType: 'ETH',
          credits: 3,
        ),
        SemesterCourseComponent(
          courseCode: 'CSE4007',
          courseType: 'ETH',
          credits: 3,
        ),
        SemesterCourseComponent(
          courseCode: 'CSE4007',
          courseType: 'ELA',
          credits: 1,
        ),
      ]);

      expect(courses, hasLength(1));
      expect(courses.single.courseCode, 'CSE4007');
      expect(courses.single.credits, 4);
    });
  });
}
