import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/features/timetable/logic/time_format.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';

/// The week as a grid with a column per class day, scrolling both ways: the
/// day header stays on top and the hours stay on the left. It opens on today
/// at the current time. Blocks show code, room, time and as much of the
/// course name as fits; tap one for the full details.
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

  static const _gutter = 30.0;
  static const _headerHeight = 52.0;
  static const _hourHeight = 80.0;
  static const _minDayWidth = 108.0;
  static const _gridTop = 8.0;

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
    final vertical = useScrollController();
    final horizontal = useScrollController();
    // The header follows the grid sideways; it doesn't scroll on its own.
    final headerScroll = useScrollController();
    useEffect(() {
      void follow() {
        if (headerScroll.hasClients) headerScroll.jumpTo(horizontal.offset);
      }

      horizontal.addListener(follow);
      return () => horizontal.removeListener(follow);
    }, [horizontal, headerScroll]);
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
    final todayIndex = days.indexOf(today);
    final minuteNow = now.value.hour * 60 + now.value.minute;
    final showNow = todayIndex >= 0 && minuteNow >= start && minuteNow <= end;

    double yOf(int minute) => (minute - start) / 60 * _hourHeight;

    // Open on today, centred, at the current time with an hour above it.
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (todayIndex >= 0 && horizontal.hasClients) {
          final position = horizontal.position;
          final dayWidth =
              (position.maxScrollExtent + position.viewportDimension) /
              days.length;
          horizontal.jumpTo(
            (todayIndex * dayWidth -
                    (position.viewportDimension - dayWidth) / 2)
                .clamp(0, position.maxScrollExtent),
          );
        }
        if (showNow && vertical.hasClients) {
          vertical.jumpTo(
            (yOf(minuteNow) - _hourHeight).clamp(
              0,
              vertical.position.maxScrollExtent,
            ),
          );
        }
      });
      return null;
    }, const []);

    final line = Container(
      height: 0.5,
      color: colors.border.withValues(alpha: 0.7),
    );

    return Padding(
      padding: const .fromLTRB(Space.sm, Space.md, Space.sm, 0),
      child: Surface(
        padding: const .fromLTRB(0, 0, Space.xs, 0),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final dayWidth = math.max(
              _minDayWidth,
              (constraints.maxWidth - _gutter) / days.length,
            );
            final gridWidth = dayWidth * days.length;

            return Column(
              children: [
                // Day header
                SizedBox(
                  height: _headerHeight,
                  child: Row(
                    children: [
                      const SizedBox(width: _gutter),
                      Expanded(
                        child: SingleChildScrollView(
                          controller: headerScroll,
                          scrollDirection: .horizontal,
                          physics: const NeverScrollableScrollPhysics(),
                          child: Row(
                            children: [
                              for (final day in days)
                                SizedBox(
                                  width: dayWidth,
                                  child: _DayHeader(
                                    name: slotDays[day - 1],
                                    date: dates[day - 1],
                                    isToday: day == today,
                                    count: daySlots[day]!.length,
                                  ),
                                ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                line,
                Expanded(
                  child: SingleChildScrollView(
                    controller: vertical,
                    padding: const .only(top: _gridTop, bottom: Space.sm),
                    child: Row(
                      crossAxisAlignment: .start,
                      children: [
                        // Hour labels
                        SizedBox(
                          width: _gutter,
                          height: gridHeight + 8,
                          child: Stack(
                            clipBehavior: .none,
                            children: [
                              for (var m = start; m <= end; m += 60)
                                Positioned(
                                  left: 0,
                                  width: _gutter - 6,
                                  top: yOf(m) - 6,
                                  child: Text(
                                    _hourLabel(m),
                                    textAlign: .right,
                                    style: numStyle(
                                      fontSize: 10,
                                      height: 1,
                                      fontWeight: .w500,
                                      color: colors.mutedForeground,
                                    ),
                                  ),
                                ),
                            ],
                          ),
                        ),
                        Expanded(
                          child: SingleChildScrollView(
                            controller: horizontal,
                            scrollDirection: .horizontal,
                            child: SizedBox(
                              width: gridWidth,
                              height: gridHeight + 8,
                              child: Stack(
                                children: [
                                  // Today's column tint
                                  if (todayIndex >= 0)
                                    Positioned(
                                      left: todayIndex * dayWidth,
                                      top: 0,
                                      width: dayWidth,
                                      height: gridHeight,
                                      child: DecoratedBox(
                                        decoration: BoxDecoration(
                                          color: colors.app.accentTone.subtle
                                              .withValues(alpha: 0.45),
                                          borderRadius: .circular(Radii.sm),
                                        ),
                                      ),
                                    ),
                                  // Hour lines
                                  for (var m = start; m <= end; m += 60)
                                    Positioned(
                                      left: 0,
                                      right: 0,
                                      top: yOf(m),
                                      child: line,
                                    ),
                                  // Class blocks
                                  for (final (i, day) in days.indexed)
                                    for (final slot in daySlots[day]!)
                                      Positioned(
                                        left: i * dayWidth + 2,
                                        width: dayWidth - 4,
                                        top:
                                            yOf(minutesOf(slot.startTime)) +
                                            1.5,
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
                                                  minutesOf(slot.endTime) <=
                                                      minuteNow),
                                          live:
                                              day == today &&
                                              minutesOf(slot.startTime) <=
                                                  minuteNow &&
                                              minutesOf(slot.endTime) >
                                                  minuteNow,
                                          onPress: () => showFSheet(
                                            context: context,
                                            side: .btt,
                                            useRootNavigator: true,
                                            builder: (context) => _ClassSheet(
                                              slot: slot,
                                              record: attendanceForSlot(
                                                attendance,
                                                slot,
                                              ),
                                            ),
                                          ),
                                        ),
                                      ),
                                  // Now line: faint across the week, solid
                                  // on today.
                                  if (showNow) ...[
                                    Positioned(
                                      left: 0,
                                      right: 0,
                                      top: yOf(minuteNow),
                                      child: Container(
                                        height: 1,
                                        color: colors.app.accent.withValues(
                                          alpha: 0.3,
                                        ),
                                      ),
                                    ),
                                    Positioned(
                                      left: todayIndex * dayWidth,
                                      width: dayWidth,
                                      top: yOf(minuteNow) - 3,
                                      height: 7,
                                      child: IgnorePointer(
                                        child: Row(
                                          children: [
                                            Dot(
                                              size: 7,
                                              color: colors.app.accent,
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
                          ),
                        ),
                      ],
                    ),
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
          padding: const .symmetric(vertical: 5),
          decoration: BoxDecoration(
            color: isToday ? colors.primary : null,
            borderRadius: .circular(Radii.md),
          ),
          child: Column(
            mainAxisSize: .min,
            children: [
              Text(
                name,
                style: TextStyle(
                  fontSize: 9.5,
                  letterSpacing: 0.6,
                  fontWeight: .w600,
                  color: isToday
                      ? colors.primaryForeground.withValues(alpha: 0.7)
                      : colors.mutedForeground,
                ),
              ),
              const SizedBox(height: 1),
              Text(
                '${date.day}',
                style: numStyle(
                  fontSize: 15,
                  height: 1.2,
                  fontWeight: .w700,
                  color: isToday ? colors.primaryForeground : colors.foreground,
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
          padding: const .fromLTRB(5, 4, 3, 3),
          decoration: BoxDecoration(
            // Opaque so grid lines don't show through.
            color: Color.alphaBlend(tone.subtle, colors.card),
            borderRadius: .circular(6),
            border: live
                ? Border.all(color: tone.base, width: 1.5)
                : Border(left: BorderSide(color: tone.base, width: 2.5)),
          ),
          child: LayoutBuilder(
            builder: (context, c) {
              final small = numStyle(
                fontSize: 9,
                height: 1.25,
                fontWeight: .w500,
                color: colors.mutedForeground,
              );
              // Lines of course name that fit under code, room and time.
              final nameLines = ((c.maxHeight - 38) / 11.5).floor();
              return Column(
                crossAxisAlignment: .start,
                children: [
                  FittedBox(
                    fit: .scaleDown,
                    alignment: .centerLeft,
                    child: Text(
                      slot.courseCode,
                      maxLines: 1,
                      style: numStyle(
                        fontSize: 10.5,
                        height: 1.2,
                        fontWeight: .w700,
                        color: colors.foreground,
                      ),
                    ),
                  ),
                  Text(
                    slot.roomNo,
                    maxLines: 1,
                    overflow: .clip,
                    style: numStyle(
                      fontSize: 9.5,
                      height: 1.25,
                      fontWeight: .w600,
                      color: tone.onSubtle,
                    ),
                  ),
                  FittedBox(
                    fit: .scaleDown,
                    alignment: .centerLeft,
                    child: Row(
                      children: [
                        if (isLab) ...[
                          Icon(
                            FLucideIcons.flaskConical,
                            size: 9,
                            color: tone.base,
                          ),
                          const SizedBox(width: 2),
                        ],
                        Text(
                          '${_clock(slot.startTime, context)}–${_clock(slot.endTime, context)}',
                          style: small,
                        ),
                      ],
                    ),
                  ),
                  if (nameLines > 0) ...[
                    const SizedBox(height: 2),
                    Text(
                      slot.name,
                      maxLines: nameLines,
                      overflow: .ellipsis,
                      style: TextStyle(
                        fontSize: 9,
                        height: 1.25,
                        color: colors.mutedForeground,
                      ),
                    ),
                  ],
                ],
              );
            },
          ),
        ),
      ),
    );
  }

  /// "9:50" from "09:50" or "2:00" from "14:00"; the grid already says
  /// morning or afternoon.
  static String _clock(String time, BuildContext context) =>
      to12H(time, context).split(' ').first;
}

class _ClassSheet extends ConsumerWidget {
  const _ClassSheet({required this.slot, required this.record});

  final TimetableSlot slot;
  final AttendanceRecord? record;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final hideSkips = !ref.watch(showSkipAdviceProvider);
    final minutes = minutesOf(slot.endTime) - minutesOf(slot.startTime);
    // Advice in sessions for labs; the x/y line below stays VTOP's own.
    final record = this.record;
    final standing = record == null
        ? null
        : AttendanceStanding.sessions(record);
    final summary = record == null ? null : AttendanceStanding.of(record);
    final tone = standing == null
        ? null
        : attendanceTone(
            context,
            safe: standing.isSafe,
            atEdge: standing.canSkip == 0,
          );

    Widget line(IconData icon, String text) => Padding(
      padding: const .only(top: Space.sm),
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
        borderRadius: const .vertical(top: Radius.circular(Radii.lg + 4)),
      ),
      child: Padding(
        padding: .fromLTRB(
          Space.lg,
          0,
          Space.lg,
          Space.xl + MediaQuery.paddingOf(context).bottom,
        ),
        child: Column(
          mainAxisSize: .min,
          crossAxisAlignment: .start,
          children: [
            const SheetHandle(),
            const SizedBox(height: Space.sm),
            Text(
              slot.name,
              style: typography.body.xl.copyWith(
                fontWeight: .w600,
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
                padding: const .all(Space.md),
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
                        style: numStyle(
                          fontSize: 12,
                          fontWeight: .w700,
                          color: tone.onSubtle,
                        ),
                      ),
                    ),
                    const SizedBox(width: Space.md),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: .start,
                        children: [
                          if (!hideSkips)
                            Text(
                              standing.advice,
                              style: typography.body.sm.copyWith(
                                fontWeight: .w600,
                                color: colors.foreground,
                              ),
                            ),
                          Text(
                            '${summary!.attended} of ${summary.total} attended',
                            style: hideSkips
                                ? typography.body.sm.copyWith(
                                    fontWeight: .w600,
                                    color: colors.foreground,
                                  )
                                : typography.body.xs.copyWith(
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
