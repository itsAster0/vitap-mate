import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/days_stack.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/timetable_card.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class WeeklyTimetableView extends StatelessWidget {
  const WeeklyTimetableView({
    super.key,
    required this.days,
    required this.slotsForDay,
  });

  final List<int> days;
  final List<TimetableSlot> Function(int day) slotsForDay;

  static const _dayNames = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  static const _timeGutterWidth = 62.0;
  static const _dayWidth = 164.0;
  static const _headerHeight = 66.0;
  static const _hourHeight = 82.0;
  static const _bottomPadding = 18.0;

  @override
  Widget build(BuildContext context) {
    final dates = getCurrentWeekDates();
    final daySlots = {for (final day in days) day: slotsForDay(day)};
    final allSlots = daySlots.values.expand((slots) => slots).toList();

    if (allSlots.isEmpty) {
      return Padding(
        padding: const EdgeInsets.all(24),
        child: Center(
          child: Text(
            'No classes this week',
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
        ),
      );
    }

    final earliestMinute = allSlots
        .map((slot) => _minutesFromMidnight(slot.startTime))
        .reduce(math.min);
    final latestMinute = allSlots
        .map((slot) => _minutesFromMidnight(slot.endTime))
        .reduce(math.max);
    final calendarStart = (earliestMinute ~/ 60) * 60;
    final calendarEnd = ((latestMinute + 59) ~/ 60) * 60;
    final calendarMinutes = math.max(60, calendarEnd - calendarStart);
    final gridHeight = calendarMinutes / 60 * _hourHeight;
    final calendarWidth = _timeGutterWidth + days.length * _dayWidth;
    final totalHeight = _headerHeight + gridHeight + _bottomPadding;
    final now = DateTime.now();

    double minuteToY(int minute) =>
        _headerHeight + (minute - calendarStart) / 60 * _hourHeight;

    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 8, 4, 4),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(14),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: context.theme.colors.background,
            borderRadius: BorderRadius.circular(14),
            border: Border.all(color: context.theme.colors.border),
          ),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: SizedBox(
              width: calendarWidth,
              height: totalHeight,
              child: Stack(
                children: [
                  for (var index = 0; index < days.length; index++)
                    if (days[index] == now.weekday)
                      Positioned(
                        left: _timeGutterWidth + index * _dayWidth,
                        top: _headerHeight,
                        width: _dayWidth,
                        height: gridHeight,
                        child: ColoredBox(
                          color: context.theme.colors.primary.withValues(
                            alpha: 0.035,
                          ),
                        ),
                      ),
                  Positioned(
                    left: 0,
                    top: 0,
                    width: calendarWidth,
                    height: _headerHeight,
                    child: ColoredBox(color: context.theme.colors.secondary),
                  ),
                  Positioned(
                    left: 0,
                    top: 0,
                    width: _timeGutterWidth,
                    height: _headerHeight,
                    child: Center(
                      child: Text(
                        'Time',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w600,
                          color: context.theme.colors.mutedForeground,
                        ),
                      ),
                    ),
                  ),
                  for (var index = 0; index < days.length; index++)
                    Positioned(
                      left: _timeGutterWidth + index * _dayWidth,
                      top: 0,
                      width: _dayWidth,
                      height: _headerHeight,
                      child: _DayHeader(
                        dayName: _dayNames[days[index] - 1],
                        date: dates[days[index] - 1],
                        isToday: days[index] == now.weekday,
                      ),
                    ),
                  for (
                    var minute = calendarStart;
                    minute <= calendarEnd;
                    minute += 30
                  )
                    Positioned(
                      left: _timeGutterWidth,
                      right: 0,
                      top: minuteToY(minute),
                      child: Container(
                        height: minute % 60 == 0 ? 1 : 0.5,
                        color: context.theme.colors.border.withValues(
                          alpha: minute % 60 == 0 ? 0.9 : 0.45,
                        ),
                      ),
                    ),
                  for (
                    var minute = calendarStart;
                    minute <= calendarEnd;
                    minute += 60
                  )
                    Positioned(
                      left: 4,
                      top: minuteToY(minute) - 8,
                      width: _timeGutterWidth - 9,
                      child: Text(
                        _formatTime(context, minute),
                        textAlign: TextAlign.right,
                        maxLines: 1,
                        style: TextStyle(
                          fontSize: 10,
                          fontWeight: FontWeight.w500,
                          color: context.theme.colors.mutedForeground,
                        ),
                      ),
                    ),
                  for (var index = 0; index <= days.length; index++)
                    Positioned(
                      left: _timeGutterWidth + index * _dayWidth,
                      top: 0,
                      width: 1,
                      height: _headerHeight + gridHeight,
                      child: ColoredBox(color: context.theme.colors.border),
                    ),
                  Positioned(
                    left: 0,
                    right: 0,
                    top: _headerHeight,
                    child: Container(
                      height: 1,
                      color: context.theme.colors.border,
                    ),
                  ),
                  for (var dayIndex = 0; dayIndex < days.length; dayIndex++)
                    for (final slot in daySlots[days[dayIndex]]!)
                      Positioned(
                        left: _timeGutterWidth + dayIndex * _dayWidth + 5,
                        top:
                            minuteToY(_minutesFromMidnight(slot.startTime)) + 3,
                        width: _dayWidth - 10,
                        height: math.max(
                          46,
                          (_minutesFromMidnight(slot.endTime) -
                                      _minutesFromMidnight(slot.startTime)) /
                                  60 *
                                  _hourHeight -
                              6,
                        ),
                        child: _CalendarClassBlock(slot: slot),
                      ),
                  if (days.contains(now.weekday) &&
                      now.hour * 60 + now.minute >= calendarStart &&
                      now.hour * 60 + now.minute <= calendarEnd)
                    _CurrentTimeIndicator(
                      left:
                          _timeGutterWidth +
                          days.indexOf(now.weekday) * _dayWidth,
                      top: minuteToY(now.hour * 60 + now.minute),
                      width: _dayWidth,
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  static int _minutesFromMidnight(String time) {
    final parts = time.split(':');
    return int.parse(parts[0]) * 60 + int.parse(parts[1]);
  }

  static String _formatTime(BuildContext context, int minutes) {
    final hour = (minutes ~/ 60).toString().padLeft(2, '0');
    final minute = (minutes % 60).toString().padLeft(2, '0');
    return to12H('$hour:$minute', context);
  }
}

class _DayHeader extends StatelessWidget {
  const _DayHeader({
    required this.dayName,
    required this.date,
    required this.isToday,
  });

  final String dayName;
  final DateTime date;
  final bool isToday;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Container(
          width: 36,
          height: 36,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: isToday
                ? context.theme.colors.primary
                : context.theme.colors.background,
            borderRadius: BorderRadius.circular(10),
            border: isToday
                ? null
                : Border.all(color: context.theme.colors.border),
          ),
          child: Text(
            date.day.toString(),
            style: TextStyle(
              fontSize: 15,
              fontWeight: FontWeight.w700,
              color: isToday
                  ? context.theme.colors.primaryForeground
                  : context.theme.colors.foreground,
            ),
          ),
        ),
        const SizedBox(width: 8),
        Text(
          dayName,
          style: TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w700,
            color: isToday
                ? context.theme.colors.primary
                : context.theme.colors.foreground,
          ),
        ),
      ],
    );
  }
}

class _CalendarClassBlock extends StatelessWidget {
  const _CalendarClassBlock({required this.slot});

  final TimetableSlot slot;

  @override
  Widget build(BuildContext context) {
    final isLab = slot.islab();
    final accent = isLab
        ? const Color(0xFF0284C7)
        : context.theme.colors.primary;
    final background = Color.alphaBlend(
      accent.withValues(alpha: isLab ? 0.22 : 0.18),
      context.theme.colors.background,
    );

    return Container(
      padding: const EdgeInsets.fromLTRB(8, 6, 7, 6),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(8),
        border: Border(left: BorderSide(color: accent, width: 3)),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final showLocation = constraints.maxHeight >= 64;
          final showType = constraints.maxHeight >= 88;

          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '${to12H(slot.startTime, context)} – ${to12H(slot.endTime, context)}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 10,
                  height: 1.1,
                  fontWeight: FontWeight.w700,
                  color: accent,
                ),
              ),
              const SizedBox(height: 4),
              Flexible(
                child: Text(
                  slot.name,
                  maxLines: showLocation ? 2 : 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: 12,
                    height: 1.15,
                    fontWeight: FontWeight.w700,
                    color: context.theme.colors.foreground,
                  ),
                ),
              ),
              if (showLocation) ...[
                const SizedBox(height: 4),
                Row(
                  children: [
                    Icon(FLucideIcons.mapPin, size: 11, color: accent),
                    const SizedBox(width: 3),
                    Expanded(
                      child: Text(
                        '${slot.block} - ${slot.roomNo}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 10,
                          height: 1,
                          color: context.theme.colors.foreground.withValues(
                            alpha: 0.72,
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ],
              if (showType) ...[
                const Spacer(),
                Text(
                  '${slot.courseCode} • ${isLab ? 'LAB' : 'LECTURE'}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontSize: 9,
                    fontWeight: FontWeight.w600,
                    color: accent,
                  ),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _CurrentTimeIndicator extends StatelessWidget {
  const _CurrentTimeIndicator({
    required this.left,
    required this.top,
    required this.width,
  });

  final double left;
  final double top;
  final double width;

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: left,
      top: top - 3,
      width: width,
      height: 7,
      child: Row(
        children: [
          Container(
            width: 7,
            height: 7,
            decoration: BoxDecoration(
              color: context.theme.colors.primary,
              shape: BoxShape.circle,
            ),
          ),
          Expanded(
            child: Container(height: 2, color: context.theme.colors.primary),
          ),
        ],
      ),
    );
  }
}
