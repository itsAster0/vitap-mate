import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/domain/attendance_projection.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';
import 'package:vitapmate/features/calendar/domain/semester_calendar.dart';
import 'package:vitapmate/features/calendar/presentation/providers/academic_calendar_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// The attendance list before VTOP posts any: each timetabled course with
/// how many classes it has left and how many can be missed at 75%, from
/// the timetable and academic calendar.
class AttendanceNotPosted extends ConsumerWidget {
  const AttendanceNotPosted({super.key, required this.fallback});

  /// Shown when there is no timetable or calendar to count from.
  final Widget fallback;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final timetable = ref.watch(timetableProvider).value;
    final calendar = ref.watch(semesterCalendarProvider);
    if (timetable == null || calendar == null || timetable.slots.isEmpty) {
      return fallback;
    }
    final now = DateTime.now();
    final exam =
        ref.watch(classesLeftUntilProvider) == ClassesLeftUntil.nextExam
        ? calendar.nextExam(now)
        : null;

    // One entry per course and kind, theory before lab, in timetable order.
    final courses = <(String, bool), String>{};
    for (final lab in [false, true]) {
      for (final slot in timetable.slots) {
        if ((slot.kind == ClassKind.lab) != lab) continue;
        courses.putIfAbsent((slot.courseCode, lab), () => slot.name);
      }
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Banner(calendar: calendar, now: now, courses: courses.length),
        const SizedBox(height: Space.md),
        for (final (i, MapEntry(key: (code, lab), value: name))
            in courses.entries.indexed)
          Padding(
            padding: const EdgeInsets.only(bottom: Space.sm + 2),
            child: EnterFade(
              index: i,
              child: _CourseBudget(
                name: name,
                lab: lab,
                next: nextClassLabel(
                  context,
                  timetable,
                  calendar,
                  code: code,
                  lab: lab,
                  now: now,
                ),
                left: AttendanceProjection.classesLeft(
                  code: code,
                  lab: lab,
                  timetable: timetable,
                  calendar: calendar,
                  now: now,
                  until: exam?.start,
                ),
                hideSkips: ref.watch(hideSkipAdviceProvider),
                examName: exam?.name,
              ),
            ),
          ),
      ],
    );
  }
}

/// "Attendance not posted yet · Classes began 14 Jul · 11 courses".
class _Banner extends StatelessWidget {
  const _Banner({
    required this.calendar,
    required this.now,
    required this.courses,
  });

  final SemesterCalendar calendar;
  final DateTime now;
  final int courses;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final first = calendar.firstClassDay;
    final today = DateTime(now.year, now.month, now.day);
    final detail = [
      if (first != null)
        '${first.isAfter(today) ? 'Classes begin' : 'Classes began'} '
            '${DateFormat('d MMM').format(first)}',
      '$courses ${courses == 1 ? 'course' : 'courses'}',
    ].join(' · ');
    return Surface(
      child: Row(
        children: [
          Icon(FLucideIcons.clock, size: 18, color: colors.mutedForeground),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Attendance not posted yet',
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
                Text(
                  detail,
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// A course card without attendance: name, kind, next class, and its
/// budget for the semester (or until [examName]).
class _CourseBudget extends StatelessWidget {
  const _CourseBudget({
    required this.name,
    required this.lab,
    required this.next,
    required this.left,
    this.examName,
    this.hideSkips = false,
  });

  final String name;
  final bool lab;
  final String? next;

  /// Classes to come, in VTOP's units (two per lab session).
  final int left;
  final String? examName;

  /// Leaves out "can miss".
  final bool hideSkips;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final base = typography.body.xs.copyWith(
      color: colors.mutedForeground,
      fontFeatures: const [FontFeature.tabularFigures()],
    );
    final strong = base.copyWith(
      fontWeight: FontWeight.w600,
      color: colors.foreground,
    );
    // Same strict 75% as the attendance cards, from a clean start.
    final canMiss = AttendanceProjection(
      standing: const AttendanceStanding(attended: 0, total: 0),
      left: left,
    ).canMiss;
    final parts = <(String?, String)>[
      if (left == 0)
        (
          null,
          examName == null
              ? 'No classes left this semester'
              : 'No classes before $examName',
        )
      else ...[
        (
          '${lab ? (left / 2).ceil() : left}',
          '${lab ? ' sessions' : ''} left'
              '${examName == null ? '' : ' till $examName'}',
        ),
        // Whole sessions only: missing one costs two classes.
        if (!hideSkips) ('${lab ? canMiss ~/ 2 : canMiss}', ' can miss'),
      ],
    ];

    return Surface(
      padding: EdgeInsets.zero,
      semanticsLabel:
          '$name, ${lab ? 'lab' : 'theory'}, attendance not posted yet',
      child: ClipRRect(
        borderRadius: BorderRadius.circular(Radii.lg - 1),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Container(width: 4, color: colors.border),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(
                    Space.md + 2,
                    Space.md + 2,
                    Space.md + 2,
                    Space.md,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        name,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: typography.body.md.copyWith(
                          height: 1.25,
                          fontWeight: FontWeight.w600,
                          color: colors.foreground,
                        ),
                      ),
                      const SizedBox(height: Space.xs + 2),
                      Row(
                        children: [
                          CourseKindBadge(isLab: lab),
                          if (next != null) ...[
                            const SizedBox(width: Space.sm),
                            Expanded(
                              child: Text(
                                'Next: $next',
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: base,
                              ),
                            ),
                          ],
                        ],
                      ),
                      const SizedBox(height: Space.md),
                      Container(height: 1, color: colors.border),
                      const SizedBox(height: Space.sm),
                      Center(
                        child: Text.rich(
                          TextSpan(
                            children: [
                              for (final (i, (value, label))
                                  in parts.indexed) ...[
                                if (i > 0) TextSpan(text: '  ·  ', style: base),
                                if (value != null)
                                  TextSpan(text: value, style: strong),
                                TextSpan(text: label, style: base),
                              ],
                            ],
                          ),
                          textAlign: TextAlign.center,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
