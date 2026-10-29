import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/calendar/logic/calendar_events.dart';
import 'package:vitapmate/features/calendar/widgets/month_events.dart';
import 'package:vitapmate/features/calendar/widgets/month_grid.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/ui/ui.dart';

DateTime _dateOnly(DateTime d) => DateTime(d.year, d.month, d.day);

/// The semester outlook, a swipeable month grid and the shown month's events.
class CalendarView extends HookWidget {
  const CalendarView({super.key, required this.data, required this.onRefresh});

  final AcademicCalendarData data;
  final Future<void> Function() onRefresh;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final now = DateTime.now();
    final today = _dateOnly(now);

    final byDate = useMemoized(() {
      final map = <DateTime, List<CalendarEntry>>{};
      for (final entry in generalEntries(data)) {
        final date = DateTime.tryParse(entry.date);
        if (date == null) continue;
        map.putIfAbsent(_dateOnly(date), () => []).add(entry);
      }
      return map;
    }, [data]);
    final semester = useMemoized(() => SemesterCalendar.of(data), [data]);
    final events = useMemoized(() => semesterEvents(byDate), [byDate]);

    // Every month from the first to the last dated entry.
    final months = useMemoized(() {
      final dates = byDate.keys.toList()..sort();
      final result = <DateTime>[];
      var month = DateTime(dates.first.year, dates.first.month);
      final last = DateTime(dates.last.year, dates.last.month);
      while (!month.isAfter(last)) {
        result.add(month);
        month = DateTime(month.year, month.month + 1);
      }
      return result;
    }, [byDate]);

    final initial = () {
      final index = months.indexWhere(
        (m) => m.year == now.year && m.month == now.month,
      );
      if (index >= 0) return index;
      return now.isBefore(months.first) ? 0 : months.length - 1;
    }();
    final page = useState(initial);
    final controller = usePageController(initialPage: initial);
    final month = months[page.value];

    void goTo(int index) => controller.animateToPage(
      index,
      duration: Motion.medium,
      curve: Curves.easeOutCubic,
    );

    return RefreshIndicator(
      onRefresh: onRefresh,
      backgroundColor: colors.background,
      color: colors.foreground,
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.lg),
        children: [
          _Outlook(calendar: semester, now: now),
          const SizedBox(height: Space.md),
          Surface(
            padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.md),
            child: Column(
              children: [
                Row(
                  children: [
                    FButton.icon(
                      variant: .ghost,
                      semanticsLabel: 'Previous month',
                      onPress: page.value > 0
                          ? () => goTo(page.value - 1)
                          : null,
                      child: const Icon(FLucideIcons.chevronLeft),
                    ),
                    Expanded(
                      child: AnimatedSwitcher(
                        duration: Motion.fast,
                        child: Text(
                          DateFormat('MMMM yyyy').format(month),
                          key: ValueKey(month),
                          textAlign: .center,
                          style: context.theme.typography.body.md.copyWith(
                            fontWeight: .w600,
                            color: colors.foreground,
                          ),
                        ),
                      ),
                    ),
                    FButton.icon(
                      variant: .ghost,
                      semanticsLabel: 'Next month',
                      onPress: page.value < months.length - 1
                          ? () => goTo(page.value + 1)
                          : null,
                      child: const Icon(FLucideIcons.chevronRight),
                    ),
                  ],
                ),
                const SizedBox(height: Space.sm),
                const WeekdayHeader(),
                AnimatedContainer(
                  duration: Motion.medium,
                  curve: Curves.easeOutCubic,
                  height: MonthGrid.heightFor(month),
                  child: PageView.builder(
                    controller: controller,
                    itemCount: months.length,
                    onPageChanged: (index) => page.value = index,
                    // The container animates to the shown month's height,
                    // so a longer neighbour is clipped mid-swipe.
                    itemBuilder: (context, index) => SingleChildScrollView(
                      physics: const NeverScrollableScrollPhysics(),
                      child: MonthGrid(
                        month: months[index],
                        byDate: byDate,
                        today: today,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: Space.sm),
                const Legend(),
              ],
            ),
          ),
          SectionHeader(title: DateFormat('MMMM').format(month)),
          AnimatedSwitcher(
            duration: Motion.medium,
            child: MonthEvents(
              key: ValueKey(month),
              month: month,
              events: events,
              today: today,
            ),
          ),
          DataUpdatedFooter(updateTime: data.updateTime.toInt()),
        ],
      ),
    );
  }
}

/// "27 class days left · Next holiday Fri 2 Oct, Mahatma Gandhi Jayanti".
class _Outlook extends StatelessWidget {
  const _Outlook({required this.calendar, required this.now});

  final SemesterCalendar calendar;
  final DateTime now;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final base = context.theme.typography.body.sm.nums(
      color: colors.mutedForeground,
    );
    final strong = base.copyWith(fontWeight: .w600, color: colors.foreground);
    final daysLeft = calendar.instructionalDaysLeft(now);
    final holiday = calendar.nextHoliday(now);
    final name = holiday == null || holiday.name == 'Holiday'
        ? ''
        : ', ${holiday.name}';
    return Padding(
      padding: const .symmetric(horizontal: 4),
      child: Text.rich(
        TextSpan(
          children: [
            if (daysLeft == 0)
              TextSpan(text: 'No class days left', style: base)
            else ...[
              TextSpan(text: '$daysLeft', style: strong),
              TextSpan(
                text: ' class ${daysLeft == 1 ? 'day' : 'days'} left',
                style: base,
              ),
            ],
            if (holiday != null) ...[
              TextSpan(text: '  ·  Next holiday ', style: base),
              TextSpan(
                text: DateFormat('EEE d MMM').format(holiday.date),
                style: strong,
              ),
              TextSpan(text: name, style: base),
            ],
          ],
        ),
      ),
    );
  }
}
