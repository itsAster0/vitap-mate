import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';
import 'package:vitapmate/features/timetable/presentation/utils/time_format.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// The whole week on one screen: a compact grid with a column per class day.
/// Blocks show course code and room; tap one for the full details.
class WeeklyTimetableView extends HookWidget {
  const WeeklyTimetableView({
    super.key,
    required this.days,
    required this.slotsForDay,
    this.attendance = const [],
  });

  final List<int> days;
  final List<TimetableSlot> Function(int day) slotsForDay;
  final List<AttendanceRecord> attendance;

  static const _dayNames = ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN'];
  static const _gutter = 30.0;
  static const _headerHeight = 52.0;
  static const _hourHeight = 58.0;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final now = useState(DateTime.now());
    useEffect(() {
      final timer = Timer.periodic(
        const Duration(minutes: 1),
        (_) => now.value = DateTime.now(),
      );
      return timer.cancel;
    }, const []);

    final dates = getCurrentWeekDates();
    final daySlots = {for (final day in days) day: slotsForDay(day)};
    final allSlots = daySlots.values.expand((slots) => slots).toList();

    if (allSlots.isEmpty) {
      return const EmptyState(
        icon: FLucideIcons.calendarX,
        title: 'No classes this week',
      );
    }

    final start =
        (allSlots.map((s) => minutesOf(s.startTime)).reduce(math.min) ~/ 60) *
        60;
    final end =
        ((allSlots.map((s) => minutesOf(s.endTime)).reduce(math.max) + 59) ~/
            60) *
        60;
    final gridHeight = math.max(60, end - start) / 60 * _hourHeight;
    final today = now.value.weekday;
    final minuteNow = now.value.hour * 60 + now.value.minute;
    final showNow =
        days.contains(today) && minuteNow >= start && minuteNow <= end;

    double yOf(int minute) => (minute - start) / 60 * _hourHeight;

    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.sm, Space.md, Space.sm, 0),
      child: Surface(
        padding: const EdgeInsets.fromLTRB(0, 0, Space.xs, Space.sm),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final dayWidth = (constraints.maxWidth - _gutter) / days.length;
            final todayIndex = days.indexOf(today);

            return Column(
              children: [
                // Day header
                SizedBox(
                  height: _headerHeight,
                  child: Row(
                    children: [
                      const SizedBox(width: _gutter),
                      for (final day in days)
                        SizedBox(
                          width: dayWidth,
                          child: _DayHeader(
                            name: _dayNames[day - 1],
                            date: dates[day - 1],
                            isToday: day == today,
                            count: daySlots[day]!.length,
                          ),
                        ),
                    ],
                  ),
                ),
                SizedBox(
                  height: gridHeight + 8,
                  child: Stack(
                    clipBehavior: Clip.none,
                    children: [
                      // Today's column tint
                      if (todayIndex >= 0)
                        Positioned(
                          left: _gutter + todayIndex * dayWidth,
                          top: 0,
                          width: dayWidth,
                          height: gridHeight,
                          child: DecoratedBox(
                            decoration: BoxDecoration(
                              color: colors.app.accentTone.subtle.withValues(
                                alpha: 0.45,
                              ),
                              borderRadius: BorderRadius.circular(Radii.sm),
                            ),
                          ),
                        ),
                      // Hour lines and labels
                      for (var m = start; m <= end; m += 60) ...[
                        Positioned(
                          left: _gutter,
                          right: 0,
                          top: yOf(m),
                          child: Container(
                            height: 0.5,
                            color: colors.border.withValues(alpha: 0.7),
                          ),
                        ),
                        Positioned(
                          left: 0,
                          width: _gutter - 6,
                          top: yOf(m) - 6,
                          child: Text(
                            _hourLabel(m),
                            textAlign: TextAlign.right,
                            style: TextStyle(
                              fontSize: 10,
                              height: 1,
                              fontWeight: FontWeight.w500,
                              color: colors.mutedForeground,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                        ),
                      ],
                      // Class blocks
                      for (final (i, day) in days.indexed)
                        for (final slot in daySlots[day]!)
                          Positioned(
                            left: _gutter + i * dayWidth + 2,
                            width: dayWidth - 4,
                            top: yOf(minutesOf(slot.startTime)) + 1.5,
                            height: math.max(
                              30,
                              yOf(minutesOf(slot.endTime)) -
                                  yOf(minutesOf(slot.startTime)) -
                                  3,
                            ),
                            child: _Block(
                              slot: slot,
                              past:
                                  day < today ||
                                  (day == today &&
                                      minutesOf(slot.endTime) <= minuteNow),
                              live:
                                  day == today &&
                                  minutesOf(slot.startTime) <= minuteNow &&
                                  minutesOf(slot.endTime) > minuteNow,
                              onPress: () => _showClass(
                                context,
                                slot,
                                attendanceForSlot(attendance, slot),
                              ),
                            ),
                          ),
                      // Now line: faint across the week, solid on today.
                      if (showNow) ...[
                        Positioned(
                          left: _gutter,
                          right: 0,
                          top: yOf(minuteNow),
                          child: Container(
                            height: 1,
                            color: colors.app.accent.withValues(alpha: 0.3),
                          ),
                        ),
                        Positioned(
                          left: _gutter + todayIndex * dayWidth - 3,
                          width: dayWidth + 3,
                          top: yOf(minuteNow) - 3,
                          height: 7,
                          child: IgnorePointer(
                            child: Row(
                              children: [
                                Container(
                                  width: 7,
                                  height: 7,
                                  decoration: BoxDecoration(
                                    color: colors.app.accent,
                                    shape: BoxShape.circle,
                                  ),
                                ),
                                Expanded(
                                  child: Container(
                                    height: 2,
                                    color: colors.app.accent,
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }

  /// 8, 9 … 12, 1, 2 — the grid reads as a school day, so no AM/PM noise.
  static String _hourLabel(int minutes) {
    final h = (minutes ~/ 60) % 12;
    return '${h == 0 ? 12 : h}';
  }
}

class _DayHeader extends StatelessWidget {
  const _DayHeader({
    required this.name,
    required this.date,
    required this.isToday,
    required this.count,
  });

  final String name;
  final DateTime date;
  final bool isToday;
  final int count;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Semantics(
      label: '$name ${date.day}, $count classes',
      excludeSemantics: true,
      child: Center(
        child: Container(
          width: 40,
          padding: const EdgeInsets.symmetric(vertical: 5),
          decoration: BoxDecoration(
            color: isToday ? colors.primary : null,
            borderRadius: BorderRadius.circular(Radii.md),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                name,
                style: TextStyle(
                  fontSize: 9.5,
                  letterSpacing: 0.6,
                  fontWeight: FontWeight.w600,
                  color: isToday
                      ? colors.primaryForeground.withValues(alpha: 0.7)
                      : colors.mutedForeground,
                ),
              ),
              const SizedBox(height: 1),
              Text(
                '${date.day}',
                style: TextStyle(
                  fontSize: 15,
                  height: 1.2,
                  fontWeight: FontWeight.w700,
                  color: isToday ? colors.primaryForeground : colors.foreground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Block extends StatelessWidget {
  const _Block({
    required this.slot,
    required this.past,
    required this.live,
    required this.onPress,
  });

  final TimetableSlot slot;
  final bool past;
  final bool live;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final isLab = slot.kind == ClassKind.lab;
    final tone = isLab ? colors.app.lab : colors.app.accentTone;

    return PressScale(
      scale: 0.95,
      semanticsLabel:
          '${slot.name}, ${to12H(slot.startTime, context)}, room ${slot.roomNo}',
      onPress: onPress,
      child: AnimatedOpacity(
        duration: Motion.medium,
        opacity: past ? 0.45 : 1,
        child: Container(
          padding: const EdgeInsets.fromLTRB(5, 4, 3, 3),
          decoration: BoxDecoration(
            // Opaque so grid lines don't show through.
            color: Color.alphaBlend(tone.subtle, colors.card),
            borderRadius: BorderRadius.circular(6),
            border: live
                ? Border.all(color: tone.base, width: 1.5)
                : Border(left: BorderSide(color: tone.base, width: 2.5)),
          ),
          child: LayoutBuilder(
            builder: (context, c) => Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                FittedBox(
                  fit: BoxFit.scaleDown,
                  alignment: Alignment.centerLeft,
                  child: Text(
                    slot.courseCode,
                    maxLines: 1,
                    style: TextStyle(
                      fontSize: 10,
                      height: 1.15,
                      fontWeight: FontWeight.w700,
                      color: colors.foreground,
                      fontFeatures: const [FontFeature.tabularFigures()],
                    ),
                  ),
                ),
                if (c.maxHeight >= 24)
                  Text(
                    slot.roomNo,
                    maxLines: 1,
                    overflow: TextOverflow.clip,
                    style: TextStyle(
                      fontSize: 9.5,
                      height: 1.2,
                      fontWeight: FontWeight.w500,
                      color: tone.onSubtle,
                      fontFeatures: const [FontFeature.tabularFigures()],
                    ),
                  ),
                if (isLab && c.maxHeight >= 44) ...[
                  const Spacer(),
                  Icon(FLucideIcons.flaskConical, size: 10, color: tone.base),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

void _showClass(
  BuildContext context,
  TimetableSlot slot,
  AttendanceRecord? record,
) {
  showFSheet(
    context: context,
    side: FLayout.btt,
    useRootNavigator: true,
    builder: (context) => _ClassSheet(slot: slot, record: record),
  );
}

class _ClassSheet extends StatelessWidget {
  const _ClassSheet({required this.slot, required this.record});

  final TimetableSlot slot;
  final AttendanceRecord? record;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final minutes = minutesOf(slot.endTime) - minutesOf(slot.startTime);
    final standing = record == null ? null : AttendanceStanding.of(record!);
    final tone = standing == null
        ? null
        : !standing.isSafe
        ? colors.app.danger
        : standing.canSkip == 0
        ? colors.app.warning
        : colors.app.success;

    Widget line(IconData icon, String text) => Padding(
      padding: const EdgeInsets.only(top: Space.sm),
      child: Row(
        children: [
          Icon(icon, size: 15, color: colors.mutedForeground),
          const SizedBox(width: Space.sm),
          Expanded(
            child: Text(
              text,
              style: typography.body.sm.copyWith(color: colors.foreground),
            ),
          ),
        ],
      ),
    );

    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.background,
        borderRadius: const BorderRadius.vertical(
          top: Radius.circular(Radii.lg + 4),
        ),
      ),
      child: Padding(
        padding: EdgeInsets.fromLTRB(
          Space.lg,
          0,
          Space.lg,
          Space.xl + MediaQuery.paddingOf(context).bottom,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const SheetHandle(),
            const SizedBox(height: Space.sm),
            Text(
              slot.name,
              style: typography.body.xl.copyWith(
                fontWeight: FontWeight.w600,
                height: 1.2,
                color: colors.foreground,
              ),
            ),
            const SizedBox(height: Space.sm),
            Row(
              children: [
                CourseKindBadge(isLab: slot.kind == ClassKind.lab),
                const SizedBox(width: Space.sm),
                Text(
                  '${slot.courseCode} · Slot ${slot.slot}',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
            const SizedBox(height: Space.sm),
            line(
              FLucideIcons.clock,
              '${to12H(slot.startTime, context)} – ${to12H(slot.endTime, context)} · ${formatMinutes(minutes)}',
            ),
            line(FLucideIcons.mapPin, 'Room ${slot.roomNo} · ${slot.block}'),
            if (slot.faculty.trim().isNotEmpty)
              line(FLucideIcons.user, facultyName(slot.faculty)),
            if (standing != null && tone != null) ...[
              const SizedBox(height: Space.lg),
              Surface(
                padding: const EdgeInsets.all(Space.md),
                onPress: () {
                  Navigator.of(context).pop();
                  showAttendanceDetails(context, record!);
                },
                semanticsLabel: 'Open attendance details',
                child: Row(
                  children: [
                    ProgressRing(
                      value: standing.displayPercent / 100,
                      color: tone.base,
                      size: 40,
                      stroke: 4,
                      marker: attendanceThreshold / 100,
                      child: Text(
                        '${standing.displayPercent.round()}',
                        style: TextStyle(
                          fontSize: 12,
                          fontWeight: FontWeight.w700,
                          color: tone.onSubtle,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                    ),
                    const SizedBox(width: Space.md),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            standing.advice,
                            style: typography.body.sm.copyWith(
                              fontWeight: FontWeight.w600,
                              color: colors.foreground,
                            ),
                          ),
                          Text(
                            '${standing.attended} of ${standing.total} attended',
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
                    ),
                    Icon(
                      FLucideIcons.chevronRight,
                      size: 16,
                      color: colors.mutedForeground,
                    ),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
