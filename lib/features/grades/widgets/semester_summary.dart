import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/gpa/gpa.dart';
import 'package:vitapmate/ui/grade_badge.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/features/grades/logic/grade_details.dart';

/// Horizontal row of semester chips; the selected one is filled.
class SemesterChips extends StatelessWidget {
  const SemesterChips({
    super.key,
    required this.semesters,
    required this.selectedId,
    required this.onSelect,
  });

  final List<SemesterInfo> semesters;
  final String selectedId;
  final ValueChanged<String> onSelect;

  /// "Winter Semester 2025-26 Freshers" → "Winter 25-26 Freshers".
  static String short(String name) => name
      .replaceAll(RegExp(r'\s+'), ' ')
      .replaceAllMapped(
        RegExp(r'\s*Semester\s+20(\d{2})-(\d{2})'),
        (m) => ' ${m.group(1)}-${m.group(2)}',
      )
      .trim();

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return SingleChildScrollView(
      scrollDirection: .horizontal,
      child: Row(
        children: [
          for (final sem in semesters)
            Padding(
              padding: const .only(right: Space.sm),
              child: PressScale(
                scale: 0.95,
                semanticsLabel: sem.name,
                onPress: () => onSelect(sem.id),
                child: AnimatedContainer(
                  duration: Motion.medium,
                  padding: const .symmetric(
                    horizontal: Space.md,
                    vertical: Space.sm - 1,
                  ),
                  decoration: BoxDecoration(
                    color: sem.id == selectedId ? colors.primary : colors.card,
                    borderRadius: .circular(Radii.pill),
                    border: .all(
                      color: sem.id == selectedId
                          ? colors.primary
                          : colors.border,
                    ),
                  ),
                  child: Text(
                    short(sem.name),
                    style: context.theme.typography.body.sm.copyWith(
                      fontWeight: .w500,
                      color: sem.id == selectedId
                          ? colors.primaryForeground
                          : colors.foreground,
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Semester GPA from the grades, weighted by credits looked up in grade
/// history. Courses without credits or a points grade (e.g. P) are left out.
class SemesterSummary extends StatelessWidget {
  const SemesterSummary({
    super.key,
    required this.courses,
    required this.creditsByCode,
    required this.onCourse,
  });

  final List<GradeCourseRecord> courses;
  final Map<String, double?> creditsByCode;
  final ValueChanged<GradeCourseRecord> onCourse;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    var credits = 0.0;
    var points = 0.0;
    final counts = <String, int>{};
    for (final c in courses) {
      final grade = Grade.tryParse(c.grade);
      final cr = creditsByCode[c.courseCode.trim().toUpperCase()];
      if (grade != null && cr != null && cr > 0) {
        credits += cr;
        points += cr * grade.points;
      }
      final g = c.grade.trim().toUpperCase();
      if (g.isNotEmpty && g != '-') counts[g] = (counts[g] ?? 0) + 1;
    }
    final gpa = credits > 0 ? points / credits : null;
    const order = ['S', 'A', 'B', 'C', 'D', 'E', 'F', 'N', 'P'];
    int rank(String g) => order.contains(g) ? order.indexOf(g) : order.length;
    final sortedCounts = counts.entries.toList()
      ..sort((a, b) => rank(a.key).compareTo(rank(b.key)));
    final label = typography.body.xs.copyWith(
      fontWeight: .w600,
      letterSpacing: 0.6,
      color: colors.mutedForeground,
    );

    return Surface(
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          Row(
            children: [
              Expanded(child: Text('SEMESTER GPA', style: label)),
              if (gpa != null) Text('${fmtNum(credits)} credits', style: label),
            ],
          ),
          Row(
            crossAxisAlignment: .end,
            children: [
              Expanded(
                child: gpa == null
                    ? Text(
                        '—',
                        style: typography.display.xl2.copyWith(
                          color: colors.mutedForeground,
                        ),
                      )
                    : CountUp(
                        value: gpa,
                        decimals: 2,
                        style: typography.display.xl2.copyWith(
                          height: 1.1,
                          fontWeight: .w600,
                          color: colors.foreground,
                        ),
                      ),
              ),
              Padding(
                padding: const .only(bottom: 4),
                child: Text(
                  gpa == null
                      ? 'Needs credits from grade history'
                      : '${courses.length} courses',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.md),
          _CourseBar(
            courses: courses,
            creditsByCode: creditsByCode,
            onCourse: onCourse,
          ),
          if (counts.isNotEmpty) ...[
            const SizedBox(height: Space.md),
            Wrap(
              spacing: Space.md,
              runSpacing: Space.xs,
              children: [
                for (final MapEntry(key: grade, value: count) in sortedCounts)
                  Row(
                    mainAxisSize: .min,
                    children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: gradeTone(context, grade).base,
                          borderRadius: .circular(2),
                        ),
                      ),
                      const SizedBox(width: Space.xs),
                      Text(
                        '$grade $count',
                        style: typography.body.xs.nums(
                          color: colors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

/// One bar for the semester: a segment per course, as wide as its credits and
/// coloured by its grade. Tap a segment to jump to that course.
class _CourseBar extends StatelessWidget {
  const _CourseBar({
    required this.courses,
    required this.creditsByCode,
    required this.onCourse,
  });

  final List<GradeCourseRecord> courses;
  final Map<String, double?> creditsByCode;
  final ValueChanged<GradeCourseRecord> onCourse;

  @override
  Widget build(BuildContext context) => TweenAnimationBuilder<double>(
    tween: Tween(begin: 0, end: 1),
    duration: Motion.slow * 2,
    curve: Curves.easeOutCubic,
    builder: (context, t, _) => Row(
      children: [
        for (final (i, c) in courses.indexed) ...[
          if (i > 0) const SizedBox(width: 3),
          Expanded(
            // Non-credit courses still get a sliver so they're visible.
            flex: math.max(
              5,
              ((creditsByCode[c.courseCode.trim().toUpperCase()] ?? 0) * 10)
                  .round(),
            ),
            child: PressScale(
              scale: 0.9,
              semanticsLabel: '${c.courseTitle}, grade ${c.grade}',
              onPress: () => onCourse(c),
              child: Padding(
                // Taller hit area than the bar itself.
                padding: const .symmetric(vertical: 6),
                child: Opacity(
                  opacity: t,
                  child: Container(
                    height: 10,
                    decoration: BoxDecoration(
                      color: gradeTone(context, c.grade.trim()).base,
                      borderRadius: .circular(3),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ],
    ),
  );
}
