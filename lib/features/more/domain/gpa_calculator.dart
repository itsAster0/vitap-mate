const gradePoints = <String, int>{
  'S': 10,
  'A': 9,
  'B': 8,
  'C': 7,
  'D': 6,
  'E': 5,
  'F': 0,
  'N': 0,
};

class GpaCourse {
  const GpaCourse({required this.credits, required this.grade});

  final double credits;
  final String grade;
}

class SemesterCourseComponent {
  const SemesterCourseComponent({
    required this.courseCode,
    required this.courseType,
    required this.credits,
  });

  final String courseCode;
  final String courseType;
  final double credits;
}

class SemesterCourseCredits {
  const SemesterCourseCredits({
    required this.courseCode,
    required this.credits,
  });

  final String courseCode;
  final double credits;
}

/// Combines theory/lab components while ignoring repeated weekly slots.
List<SemesterCourseCredits> combineSemesterCourseCredits(
  Iterable<SemesterCourseComponent> components,
) {
  final creditsByCourse = <String, double>{};
  final seenComponents = <String>{};

  for (final component in components) {
    final code = component.courseCode.trim().toUpperCase();
    final type = component.courseType.trim().toUpperCase();
    if (code.isEmpty || component.credits <= 0) continue;
    if (!seenComponents.add('$code\u0000$type')) continue;
    creditsByCourse.update(
      code,
      (credits) => credits + component.credits,
      ifAbsent: () => component.credits,
    );
  }

  return [
    for (final entry in creditsByCourse.entries)
      SemesterCourseCredits(courseCode: entry.key, credits: entry.value),
  ];
}

double gradePointFor(String grade) =>
    (gradePoints[grade.trim().toUpperCase()] ?? 0).toDouble();

double calculateSemesterGpa(Iterable<GpaCourse> courses) {
  var totalCredits = 0.0;
  var weightedPoints = 0.0;

  for (final course in courses) {
    if (course.credits <= 0) continue;
    totalCredits += course.credits;
    weightedPoints += course.credits * gradePointFor(course.grade);
  }

  return totalCredits == 0 ? 0 : weightedPoints / totalCredits;
}

double calculateProjectedCgpa({
  required double currentCgpa,
  required double completedCredits,
  required Iterable<GpaCourse> plannedCourses,
}) {
  final safeCgpa = currentCgpa.clamp(0.0, 10.0);
  final safeCompletedCredits = completedCredits < 0 ? 0.0 : completedCredits;
  var plannedCredits = 0.0;
  var plannedPoints = 0.0;

  for (final course in plannedCourses) {
    if (course.credits <= 0) continue;
    plannedCredits += course.credits;
    plannedPoints += course.credits * gradePointFor(course.grade);
  }

  final totalCredits = safeCompletedCredits + plannedCredits;
  if (totalCredits == 0) return 0;

  return (safeCgpa * safeCompletedCredits + plannedPoints) / totalCredits;
}

double? parseCredits(String value) {
  final normalized = value.trim().replaceAll(',', '.');
  final match = RegExp(r'\d+(?:\.\d+)?').firstMatch(normalized);
  if (match == null) return null;
  return double.tryParse(match.group(0)!);
}
