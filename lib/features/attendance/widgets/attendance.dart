import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/ui/extention.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/attendance/logic/attendance_projection.dart';
import 'package:vitapmate/features/attendance/widgets/attendance_table.dart';
import 'package:vitapmate/features/attendance/widgets/course_card.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';

/// One course in the attendance list: ring, name, and what to do next.
class AttendanceCard extends ConsumerWidget {
  const AttendanceCard({super.key, required this.record, required this.index});

  final AttendanceRecord record;
  final int index;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    // The x/y and percentage are VTOP's own; skipping is worked out in
    // sessions for labs, the same as the detail sheet.
    final standing = AttendanceStanding.of(record);
    final plan = AttendanceStanding.sessions(record);
    final tone = attendanceTone(
      context,
      safe: plan.isSafe,
      atEdge: plan.canSkip == 0,
    );
    final (code, name) = formateName(record.courseName);
    final isLab = record.islab();
    final timetable = ref.watch(timetableProvider).value;
    final calendar = ref.watch(semesterCalendarProvider);
    final history = ref
        .watch(cachedFullAttendanceProvider(record.courseType, record.courseId))
        .value;
    final sync = history == null ? null : HistorySync.of(record, history);
    final outOfSync = sync?.inSync == false;
    final now = DateTime.now();
    final next = nextClassLabel(
      context,
      timetable,
      calendar,
      code: courseCodeOf(record),
      lab: isLab,
      now: now,
    );
    // In "next exam" mode, count to the next CAT/FAT; once the FAT has
    // begun there is no next exam and it counts to the end as usual.
    final exam =
        calendar != null &&
            ref.watch(classesLeftUntilProvider) == ClassesLeftUntil.nextExam
        ? calendar.nextExam(now)
        : null;
    final classes = timetable == null || calendar == null
        ? null
        : AttendanceProjection.of(
            record: record,
            timetable: timetable,
            calendar: calendar,
            now: now,
            until: exam?.start,
            countedThrough: sync?.countedThrough,
          );
    final projection = isLab ? classes?.inSessions(record) : classes;
    final hideSkips = !ref.watch(showSkipAdviceProvider);
    final unposted = projection?.unposted ?? 0;

    final pct = standing.displayPercent;
    return StripeCard(
      stripe: tone.base,
      semanticsLabel:
          '$name, ${pct.round()} percent, ${standing.attended} of ${standing.total} attended${outOfSync ? ', history not in sync' : ''}${unposted > 0 ? ', $unposted held but not posted yet' : ''}${hideSkips ? '' : ', ${plan.advice}'}',
      onPress: () => showAttendanceDetails(context, record),
      children: [
        Row(
          crossAxisAlignment: .start,
          children: [
            Expanded(
              child: Text(
                name.trim().isEmpty ? record.courseName : name.trim(),
                maxLines: 2,
                overflow: .ellipsis,
                style: typography.body.md.copyWith(
                  height: 1.25,
                  fontWeight: .w600,
                  color: colors.foreground,
                ),
              ),
            ),
            const SizedBox(width: Space.md),
            CountUp(
              value: pct,
              suffix: '%',
              style: typography.body.lg.copyWith(
                height: 1.2,
                fontWeight: .w500,
                color: tone.base,
              ),
            ),
          ],
        ),
        const SizedBox(height: Space.xs + 2),
        // Type, and when the course meets next (the code if it has no
        // upcoming class). Spans the full card width, not just the title's
        // column beside the percentage.
        Row(
          children: [
            CourseKindBadge(isLab: isLab),
            const SizedBox(width: Space.sm),
            Expanded(
              child: Text(
                next == null ? code.trim() : 'Next: $next',
                maxLines: 1,
                overflow: .ellipsis,
                style: typography.body.xs.nums(color: colors.mutedForeground),
              ),
            ),
          ],
        ),
        const SizedBox(height: Space.md),
        SkipMeter(percent: pct, tone: tone),
        const SizedBox(height: Space.md),
        Row(
          crossAxisAlignment: .end,
          children: [
            // "22/24 attended", with a warning dot when the class history
            // disagrees with these counts.
            Row(
              mainAxisSize: .min,
              children: [
                Text(
                  '${standing.attended}/${standing.total} attended',
                  style: typography.body.sm.nums(color: colors.mutedForeground),
                ),
                if (outOfSync) ...[
                  const SizedBox(width: Space.xs + 2),
                  Dot(color: colors.app.warning.base),
                ],
              ],
            ),
            if (!hideSkips) ...[
              const Spacer(),
              // Tinted when there's no room to skip.
              Text(
                plan.advice,
                style: typography.body.sm.copyWith(
                  fontWeight: .w500,
                  color: plan.isSafe && plan.canSkip > 0
                      ? colors.mutedForeground
                      : tone.onSubtle,
                ),
              ),
            ],
          ],
        ),
        if (projection != null) ...[
          const SizedBox(height: Space.md),
          _outlook(
            projection,
            tone,
            examName: exam?.name,
            hideSkips: hideSkips,
          ),
        ],
      ],
    );
  }
}

/// Where the course ends up by the last class (or by [examName]): "1+16
/// left · 8 to spare · 95% max", where the muted "1+" is classes held but
/// not posted yet. Lab projections come in sessions
/// ([AttendanceProjection.inSessions]). [hideSkips] leaves out "to spare" /
/// "needed" / "short of 75%".
Widget _outlook(
  AttendanceProjection projection,
  Tone tone, {
  String? examName,
  required bool hideSkips,
}) {
  final AttendanceProjection(:left, :mustAttend, :unposted) = projection;
  return OutlookLine(
    alert: left > 0 && mustAttend == null ? tone.onSubtle : null,
    [
      if (left == 0)
        (
          null,
          null,
          examName == null
              ? 'No classes left this semester'
              : 'No classes before $examName',
        )
      else ...[
        (
          unposted > 0 ? '$unposted+' : null,
          '${projection.upcoming}',
          examName == null ? ' left' : ' left till $examName',
        ),
        if (!hideSkips)
          if (mustAttend == null)
            (null, null, 'short of 75%')
          else if (!projection.standing.isSafe)
            (null, '$mustAttend', ' needed')
          else
            (null, '${projection.canMiss}', ' to spare'),
        (null, '${projection.bestPercent.floor()}%', ' max'),
      ],
    ],
  );
}

/// When course [code] ([lab] or theory) meets next per the timetable —
/// "Today 2:00 PM", "Tomorrow 8:00 AM", "Mon 2:00 PM", or "Sat 3 Oct 8:00
/// AM" a week or more out — or null if it isn't scheduled. With a
/// [calendar], days without classes (holidays, exams, labs after the LAB
/// FAT) are skipped.
String? nextClassLabel(
  BuildContext context,
  TimetableData? data,
  SemesterCalendar? calendar, {
  required String code,
  required bool lab,
  required DateTime now,
}) {
  if (data == null) return null;
  final nowMinute = now.hour * 60 + now.minute;
  // A week ahead without a calendar (offset 7 is the same weekday next
  // week); with one, far enough to get past exam weeks and breaks.
  final horizon = calendar == null ? 7 : 28;
  for (var offset = 0; offset <= horizon; offset++) {
    final date = DateTime(now.year, now.month, now.day + offset);
    if (calendar != null && !calendar.holdsClasses(date, lab: lab)) continue;
    final starts = [
      for (final s in data.slots)
        if (s.day == slotDays[date.weekday - 1] &&
            s.courseCode == code &&
            (s.kind == ClassKind.lab) == lab &&
            (offset > 0 || minutesOf(s.startTime) > nowMinute))
          s.startTime,
    ]..sort((a, b) => minutesOf(a).compareTo(minutesOf(b)));
    if (starts.isEmpty) continue;
    final day = switch (offset) {
      0 => 'Today',
      1 => 'Tomorrow',
      // A week or more out: add the date so it isn't read as this week.
      >= 7 => DateFormat('EEE d MMM').format(date),
      _ => DateFormat('EEE').format(date),
    };
    return '$day ${to12H(starts.first, context)}';
  }
  return null;
}

void showAttendanceDetails(BuildContext context, AttendanceRecord record) {
  showFSheet(
    context: context,
    side: .btt,
    // Above the tab bar, not inside the tab.
    useRootNavigator: true,
    mainAxisMaxRatio: 0.88,
    builder: (context) => SizedBox(
      height: MediaQuery.sizeOf(context).height * 0.88,
      child: AttendanceDetailSheet(record: record),
    ),
  );
}

/// Splits "CODE - Name - Type" into (code, name).
(String, String) formateName(String name) => switch (name.split('-')) {
  [final code, final first, final second, _, ...] => (code, '$first-$second'),
  [final code, final first, ...] => (code, first),
  _ => ('', name),
};
