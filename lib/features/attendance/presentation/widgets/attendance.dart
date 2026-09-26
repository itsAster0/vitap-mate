import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_table.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/utils/time_format.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// One course in the attendance list: ring, name, and what to do next.
class AttendanceCard extends ConsumerWidget {
  const AttendanceCard({super.key, required this.record, required this.index});

  final AttendanceRecord record;
  final int index;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final standing = AttendanceStanding.of(record);
    final tone = attendanceTone(
      context,
      safe: standing.isSafe,
      atEdge: standing.canSkip == 0,
    );
    final (code, name) = formateName(record.courseName);
    final isLab = record.islab();
    final next = _nextClassLabel(
      context,
      ref.watch(timetableProvider).value,
      record,
      DateTime.now(),
    );

    final pct = standing.displayPercent;
    return Surface(
      padding: EdgeInsets.zero,
      semanticsLabel:
          '$name, ${pct.round()} percent, ${standing.attended} of ${standing.total} attended, ${standing.advice}',
      onPress: () => showAttendanceDetails(context, record),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(Radii.lg - 1),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Container(width: 4, color: tone.base),
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
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  name.trim().isEmpty
                                      ? record.courseName
                                      : name.trim(),
                                  maxLines: 2,
                                  overflow: TextOverflow.ellipsis,
                                  style: typography.body.md.copyWith(
                                    height: 1.25,
                                    fontWeight: FontWeight.w600,
                                    color: colors.foreground,
                                  ),
                                ),
                                const SizedBox(height: Space.xs + 2),
                                // Type, and when the course meets next (the
                                // code if it has no upcoming class).
                                Row(
                                  children: [
                                    CourseKindBadge(isLab: isLab),
                                    const SizedBox(width: Space.sm),
                                    Expanded(
                                      child: Text(
                                        next == null
                                            ? code.trim()
                                            : 'Next: $next',
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: typography.body.xs.copyWith(
                                          color: colors.mutedForeground,
                                          fontFeatures: const [
                                            FontFeature.tabularFigures(),
                                          ],
                                        ),
                                      ),
                                    ),
                                  ],
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(width: Space.md),
                          CountUp(
                            value: pct,
                            suffix: '%',
                            style: typography.body.lg.copyWith(
                              height: 1.2,
                              fontWeight: FontWeight.w500,
                              color: tone.base,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: Space.md),
                      SkipMeter(percent: pct, tone: tone),
                      const SizedBox(height: Space.md),
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.end,
                        children: [
                          _ClassCount(standing: standing),
                          const Spacer(),
                          _Advice(standing: standing, tone: tone),
                        ],
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

/// "22/24 attended".
class _ClassCount extends StatelessWidget {
  const _ClassCount({required this.standing});

  final AttendanceStanding standing;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Text(
      '${standing.attended}/${standing.total} attended',
      style: typography.body.sm.copyWith(
        color: colors.mutedForeground,
        fontFeatures: const [FontFeature.tabularFigures()],
      ),
    );
  }
}

/// "Can skip 5" / "Attend 2", tinted when there's no room to skip.
class _Advice extends StatelessWidget {
  const _Advice({required this.standing, required this.tone});

  final AttendanceStanding standing;
  final Tone tone;

  @override
  Widget build(BuildContext context) {
    return Text(
      standing.advice,
      style: context.theme.typography.body.sm.copyWith(
        fontWeight: FontWeight.w500,
        color: standing.isSafe && standing.canSkip > 0
            ? context.theme.colors.mutedForeground
            : tone.onSubtle,
      ),
    );
  }
}

const _slotDays = ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN'];

/// When [record]'s course meets next per the timetable — "Today 2:00 PM",
/// "Tomorrow 8:00 AM", "Mon 2:00 PM", or "Sat 3 Oct 8:00 AM" a week out —
/// or null if it isn't scheduled.
String? _nextClassLabel(
  BuildContext context,
  TimetableData? data,
  AttendanceRecord record,
  DateTime now,
) {
  if (data == null) return null;
  final code = courseCodeOf(record);
  final lab = record.islab();
  final nowMinute = now.hour * 60 + now.minute;
  // Up to a week ahead; offset 7 is the same weekday next week.
  for (var offset = 0; offset <= 7; offset++) {
    final weekday = (now.weekday - 1 + offset) % 7 + 1;
    final slots =
        data.slots
            .where(
              (s) =>
                  s.day == _slotDays[weekday - 1] &&
                  s.courseCode == code &&
                  (s.kind == ClassKind.lab) == lab &&
                  (offset > 0 || minutesOf(s.startTime) > nowMinute),
            )
            .toList()
          ..sort(
            (a, b) => minutesOf(a.startTime).compareTo(minutesOf(b.startTime)),
          );
    if (slots.isEmpty) continue;
    final day = switch (offset) {
      0 => 'Today',
      1 => 'Tomorrow',
      // Same weekday as today: add the date so it isn't read as today.
      7 => DateFormat('EEE d MMM').format(now.add(Duration(days: offset))),
      _ => DateFormat('EEE').format(now.add(Duration(days: offset))),
    };
    return '$day ${to12H(slots.first.startTime, context)}';
  }
  return null;
}

void showAttendanceDetails(BuildContext context, AttendanceRecord record) {
  showFSheet(
    context: context,
    side: FLayout.btt,
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
(String, String) formateName(String name) {
  final splitName = name.split("-");
  if (splitName.length < 2) return ("", name);
  var nName = splitName[1];
  if (splitName.length > 3) {
    nName += "-${splitName[2]}";
  }

  return (splitName[0], nName);
}
