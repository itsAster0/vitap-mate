enum Grade {
  s(10),
  a(9),
  b(8),
  c(7),
  d(6),
  e(5),
  f(0),
  n(0);

  const Grade(this.points);

  final int points;

  String get label => name.toUpperCase();

  static Grade? tryParse(String raw) {
    final normalized = raw.trim().toUpperCase();
    for (final grade in values) {
      if (grade.label == normalized) return grade;
    }
    return null;
  }
}

final class Credits {
  const Credits._(this.value);

  final double value;

  factory Credits(double value) {
    if (!value.isFinite || value <= 0) {
      throw ArgumentError.value(value, 'value', 'must be finite and positive');
    }
    return Credits._(value);
  }

  static Credits? tryParse(String raw) {
    final normalized = raw.trim().replaceAll(',', '.');
    final match = RegExp(r'\d+(?:\.\d+)?').firstMatch(normalized);
    final parsed = match == null ? null : double.tryParse(match.group(0)!);
    return parsed == null || parsed <= 0 ? null : Credits._(parsed);
  }
}

final class Cgpa {
  const Cgpa._(this.value);

  final double value;

  factory Cgpa(double value) {
    if (!value.isFinite || value < 0 || value > 10) {
      throw ArgumentError.value(value, 'value', 'must be between 0 and 10');
    }
    return Cgpa._(value);
  }

  static Cgpa? tryParse(String raw) {
    final parsed = double.tryParse(raw.trim().replaceAll(',', '.'));
    return parsed == null || parsed < 0 || parsed > 10 ? null : Cgpa._(parsed);
  }
}

class GpaCourse {
  const GpaCourse({required this.credits, required this.grade});

  final Credits credits;
  final Grade grade;
}

class SemesterCourseComponent {
  const SemesterCourseComponent({
    required this.courseCode,
    required this.courseType,
    required this.credits,
  });

  final String courseCode;
  final String courseType;
  final Credits credits;
}

class SemesterCourseCredits {
  const SemesterCourseCredits({
    required this.courseCode,
    required this.credits,
  });

  final String courseCode;
  final Credits credits;
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
    if (code.isEmpty) continue;
    if (!seenComponents.add('$code\u0000$type')) continue;
    creditsByCourse.update(
      code,
      (credits) => credits + component.credits.value,
      ifAbsent: () => component.credits.value,
    );
  }

  return [
    for (final entry in creditsByCourse.entries)
      SemesterCourseCredits(
        courseCode: entry.key,
        credits: Credits(entry.value),
      ),
  ];
}

double gradePointFor(Grade grade) => grade.points.toDouble();

double calculateSemesterGpa(Iterable<GpaCourse> courses) {
  var totalCredits = 0.0;
  var weightedPoints = 0.0;

  for (final course in courses) {
    totalCredits += course.credits.value;
    weightedPoints += course.credits.value * gradePointFor(course.grade);
  }

  return totalCredits == 0 ? 0 : weightedPoints / totalCredits;
}

double calculateProjectedCgpa({
  required Cgpa currentCgpa,
  required double completedCredits,
  required Iterable<GpaCourse> plannedCourses,
}) {
  if (!completedCredits.isFinite || completedCredits < 0) {
    throw ArgumentError.value(
      completedCredits,
      'completedCredits',
      'must be finite and non-negative',
    );
  }
  var plannedCredits = 0.0;
  var plannedPoints = 0.0;

  for (final course in plannedCourses) {
    plannedCredits += course.credits.value;
    plannedPoints += course.credits.value * gradePointFor(course.grade);
  }

  final totalCredits = completedCredits + plannedCredits;
  if (totalCredits == 0) return 0;

  return (currentCgpa.value * completedCredits + plannedPoints) / totalCredits;
}

Credits? parseCredits(String value) => Credits.tryParse(value);
