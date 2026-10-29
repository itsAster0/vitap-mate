import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/calendar/logic/calendar_events.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/ui/ui.dart';

const _weekdays = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];

/// The weekday initials above the month grid, Sunday first.
class WeekdayHeader extends StatelessWidget {
  const WeekdayHeader({super.key});

  @override
  Widget build(BuildContext context) {
    final style = context.theme.typography.body.xs.copyWith(
      color: context.theme.colors.mutedForeground,
      fontWeight: .w600,
    );
    return Row(
      children: [
        for (final day in _weekdays)
          Expanded(
            child: Text(day, textAlign: .center, style: style),
          ),
      ],
    );
  }
}

/// The colour of a mark's dot, or null for a plain class day.
Color? markColor(BuildContext context, CalendarMark mark) {
  final app = context.theme.colors.app;
  return switch (mark) {
    CalendarMark.holiday => app.success.base,
    CalendarMark.exam => app.warning.base,
    CalendarMark.labFat => app.lab.base,
    CalendarMark.special => app.accent,
    CalendarMark.noClasses => context.theme.colors.mutedForeground,
    CalendarMark.classes => null,
  };
}

/// The month's weeks from the Sunday on or before the 1st, like VTOP's grid.
class MonthGrid extends StatelessWidget {
  const MonthGrid({
    super.key,
    required this.month,
    required this.byDate,
    required this.today,
  });

  static const _cellHeight = 44.0;

  /// Weeks the month spans, Sunday first (4 to 6).
  static int weeksIn(DateTime month) {
    final lead = month.weekday % 7;
    final days = DateTime(month.year, month.month + 1, 0).day;
    return ((lead + days) / 7).ceil();
  }

  static double heightFor(DateTime month) => _cellHeight * weeksIn(month);

  final DateTime month;
  final Map<DateTime, List<CalendarEntry>> byDate;
  final DateTime today;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final start = DateTime(month.year, month.month, 1 - month.weekday % 7);

    Widget cell(DateTime day) {
      final inMonth = day.month == month.month;
      if (!inMonth) return const SizedBox(height: _cellHeight);
      final isToday = day == today;
      final isPast = day.isBefore(today);
      final mark = dayMark(byDate[day]);
      final dot = mark == null ? null : markColor(context, mark);
      final sunday = day.weekday == DateTime.sunday;
      final off =
          sunday ||
          mark == CalendarMark.holiday ||
          mark == CalendarMark.noClasses;

      return SizedBox(
        height: _cellHeight,
        child: Column(
          mainAxisAlignment: .center,
          children: [
            Container(
              width: 30,
              height: 26,
              alignment: .center,
              decoration: isToday
                  ? BoxDecoration(
                      border: .all(color: colors.app.accent, width: 1.5),
                      borderRadius: .circular(Radii.sm),
                    )
                  : null,
              child: Text(
                '${day.day}',
                style: typography.body.sm.nums(
                  fontWeight: isToday ? FontWeight.w700 : FontWeight.w500,
                  color: off || isPast
                      ? colors.mutedForeground
                      : colors.foreground,
                ),
              ),
            ),
            const SizedBox(height: 3),
            // Sundays are always off; a dot there would only add noise.
            if (dot != null && !(sunday && mark == CalendarMark.holiday))
              Opacity(
                opacity: isPast ? 0.45 : 1,
                child: Container(
                  width: 5,
                  height: 5,
                  decoration: BoxDecoration(
                    shape: .circle,
                    color: mark == CalendarMark.noClasses ? null : dot,
                    border: mark == CalendarMark.noClasses
                        ? Border.all(color: dot, width: 1)
                        : null,
                  ),
                ),
              )
            else
              const SizedBox(height: 5),
          ],
        ),
      );
    }

    return Column(
      children: [
        for (var week = 0; week < weeksIn(month); week++)
          Row(
            children: [
              for (var d = 0; d < 7; d++)
                Expanded(
                  child: cell(
                    DateTime(start.year, start.month, start.day + week * 7 + d),
                  ),
                ),
            ],
          ),
      ],
    );
  }
}

/// What each dot colour on the month grid means.
class Legend extends StatelessWidget {
  const Legend({super.key});

  @override
  Widget build(BuildContext context) {
    final style = context.theme.typography.body.xs.copyWith(
      color: context.theme.colors.mutedForeground,
    );
    Widget item(CalendarMark mark, String label) {
      final color = markColor(context, mark)!;
      final hollow = mark == CalendarMark.noClasses;
      return Row(
        mainAxisSize: .min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              shape: .circle,
              color: hollow ? null : color,
              border: hollow ? Border.all(color: color, width: 1) : null,
            ),
          ),
          const SizedBox(width: 5),
          Text(label, style: style),
        ],
      );
    }

    return Wrap(
      alignment: .center,
      spacing: Space.md,
      runSpacing: Space.xs,
      children: [
        item(CalendarMark.holiday, 'Holiday'),
        item(CalendarMark.exam, 'Exam'),
        item(CalendarMark.labFat, 'Lab FAT'),
        item(CalendarMark.noClasses, 'No classes'),
        item(CalendarMark.special, 'Event'),
      ],
    );
  }
}
