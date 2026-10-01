import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/calendar/domain/semester_calendar.dart';
import 'package:vitapmate/features/calendar/presentation/providers/academic_calendar_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AcademicCalendarPage extends HookConsumerWidget {
  const AcademicCalendarPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> refresh() async {
      try {
        await ref.read(academicCalendarProvider.notifier).refresh();
      } catch (e) {
        log('$e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    // Like the other screens, fetch on open when Auto Refresh is on.
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!context.mounted || !await isAutoRefreshEnabled(ref)) return;
        ref.read(academicCalendarProvider.notifier).refresh().catchError((
          e,
          st,
        ) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });
      return null;
    }, const []);

    final calendar = ref.watch(academicCalendarProvider);
    return ScreenRefresh(
      onRefresh: refresh,
      tasks: const ['vtop_fetchAcademicCalendar'],
      child: AnimatedSwitcher(
        duration: Motion.medium,
        child: calendar.when(
          skipLoadingOnRefresh: true,
          skipLoadingOnReload: true,
          data: (data) => data.entries.isEmpty
              ? EmptyState(
                  key: const ValueKey('empty'),
                  icon: FLucideIcons.calendarX,
                  title: 'No calendar yet',
                  message: "VTOP hasn't published this semester's calendar.",
                  action: FButton(
                    variant: FButtonVariant.outline,
                    mainAxisSize: MainAxisSize.min,
                    onPress: refresh,
                    child: const Text('Check again'),
                  ),
                )
              : _CalendarView(
                  key: const ValueKey('data'),
                  data: data,
                  onRefresh: refresh,
                ),
          error: (e, _) => EmptyState(
            key: const ValueKey('error'),
            icon: FLucideIcons.cloudAlert,
            title: "Couldn't load the calendar",
            message: commonErrorMessage(e),
            action: FButton(
              variant: FButtonVariant.outline,
              mainAxisSize: MainAxisSize.min,
              onPress: refresh,
              child: const Text('Try again'),
            ),
          ),
          loading: () => const Padding(
            key: ValueKey('loading'),
            padding: EdgeInsets.all(Space.sm),
            child: Column(
              children: [
                Skeleton(height: 20, radius: Radii.sm),
                SizedBox(height: Space.md),
                Skeleton(height: 340, radius: Radii.lg),
                SizedBox(height: Space.lg),
                SkeletonList(count: 4, height: 52),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

DateTime _dateOnly(DateTime d) => DateTime(d.year, d.month, d.day);

class _CalendarView extends HookWidget {
  const _CalendarView({super.key, required this.data, required this.onRefresh});

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
    final events = useMemoized(() => _semesterEvents(byDate), [byDate]);

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
        padding: const EdgeInsets.fromLTRB(
          Space.sm,
          Space.sm,
          Space.sm,
          Space.lg,
        ),
        children: [
          _Outlook(calendar: semester, now: now),
          const SizedBox(height: Space.md),
          Surface(
            padding: const EdgeInsets.fromLTRB(
              Space.sm,
              Space.sm,
              Space.sm,
              Space.md,
            ),
            child: Column(
              children: [
                Row(
                  children: [
                    FButton.icon(
                      variant: FButtonVariant.ghost,
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
                          textAlign: TextAlign.center,
                          style: context.theme.typography.body.md.copyWith(
                            fontWeight: FontWeight.w600,
                            color: colors.foreground,
                          ),
                        ),
                      ),
                    ),
                    FButton.icon(
                      variant: FButtonVariant.ghost,
                      semanticsLabel: 'Next month',
                      onPress: page.value < months.length - 1
                          ? () => goTo(page.value + 1)
                          : null,
                      child: const Icon(FLucideIcons.chevronRight),
                    ),
                  ],
                ),
                const SizedBox(height: Space.sm),
                const _WeekdayHeader(),
                AnimatedContainer(
                  duration: Motion.medium,
                  curve: Curves.easeOutCubic,
                  height: _MonthGrid.heightFor(month),
                  child: PageView.builder(
                    controller: controller,
                    itemCount: months.length,
                    onPageChanged: (index) => page.value = index,
                    // The container animates to the shown month's height,
                    // so a longer neighbour is clipped mid-swipe.
                    itemBuilder: (context, index) => SingleChildScrollView(
                      physics: const NeverScrollableScrollPhysics(),
                      child: _MonthGrid(
                        month: months[index],
                        byDate: byDate,
                        today: today,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: Space.sm),
                const _Legend(),
              ],
            ),
          ),
          SectionHeader(title: DateFormat('MMMM').format(month)),
          AnimatedSwitcher(
            duration: Motion.medium,
            child: _MonthEvents(
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
    final base = context.theme.typography.body.sm.copyWith(
      color: colors.mutedForeground,
      fontFeatures: const [FontFeature.tabularFigures()],
    );
    final strong = base.copyWith(
      fontWeight: FontWeight.w600,
      color: colors.foreground,
    );
    final daysLeft = calendar.instructionalDaysLeft(now);
    final holiday = calendar.nextHoliday(now);
    final name = holiday == null || holiday.name == 'Holiday'
        ? ''
        : ', ${holiday.name}';
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
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

const _weekdays = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];

class _WeekdayHeader extends StatelessWidget {
  const _WeekdayHeader();

  @override
  Widget build(BuildContext context) {
    final style = context.theme.typography.body.xs.copyWith(
      color: context.theme.colors.mutedForeground,
      fontWeight: FontWeight.w600,
    );
    return Row(
      children: [
        for (final day in _weekdays)
          Expanded(
            child: Text(day, textAlign: TextAlign.center, style: style),
          ),
      ],
    );
  }
}

/// The colour of a mark's dot, or null for a plain class day.
Color? _markColor(BuildContext context, CalendarMark mark) {
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

/// The most notable mark among a day's entries.
CalendarMark? _dayMark(List<CalendarEntry>? entries) {
  if (entries == null || entries.isEmpty) return null;
  return entries.map(markOf).reduce((a, b) => a.index <= b.index ? a : b);
}

/// The month's weeks from the Sunday on or before the 1st, like VTOP's grid.
class _MonthGrid extends StatelessWidget {
  const _MonthGrid({
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
      final mark = _dayMark(byDate[day]);
      final dot = mark == null ? null : _markColor(context, mark);
      final sunday = day.weekday == DateTime.sunday;
      final off =
          sunday ||
          mark == CalendarMark.holiday ||
          mark == CalendarMark.noClasses;

      return SizedBox(
        height: _cellHeight,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Container(
              width: 30,
              height: 26,
              alignment: Alignment.center,
              decoration: isToday
                  ? BoxDecoration(
                      border: Border.all(color: colors.app.accent, width: 1.5),
                      borderRadius: BorderRadius.circular(Radii.sm),
                    )
                  : null,
              child: Text(
                '${day.day}',
                style: typography.body.sm.copyWith(
                  fontWeight: isToday ? FontWeight.w700 : FontWeight.w500,
                  color: off || isPast
                      ? colors.mutedForeground
                      : colors.foreground,
                  fontFeatures: const [FontFeature.tabularFigures()],
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
                    shape: BoxShape.circle,
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

class _Legend extends StatelessWidget {
  const _Legend();

  @override
  Widget build(BuildContext context) {
    final style = context.theme.typography.body.xs.copyWith(
      color: context.theme.colors.mutedForeground,
    );
    Widget item(CalendarMark mark, String label) {
      final color = _markColor(context, mark)!;
      final hollow = mark == CalendarMark.noClasses;
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
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
      alignment: WrapAlignment.center,
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

/// One row of the month's list: a single day, or a run of listed days with
/// the same title (an exam, the lab FAT). A run is broken only by a class
/// day, so holidays inside it and month ends do not split it.
class _Event {
  _Event(this.first, this.entry) : last = first, parts = [_Part(first, entry)];

  final DateTime first;
  DateTime last;

  /// The entry naming the row; for a run of days off, its most telling day.
  CalendarEntry entry;

  bool get isDayOff => _isDayOff(markOf(entry));

  /// Calendar days the row spans, ends included.
  int get days => last.difference(first).inDays + 1;

  /// The reasons inside a run of days off, each over its own days.
  final List<_Part> parts;
}

/// One reason inside a run of days off ("Holiday" on Sunday, "Vijaya
/// Dashami / Dussehra" on Tuesday).
class _Part {
  _Part(this.first, this.entry) : last = first;

  final DateTime first;
  DateTime last;
  final CalendarEntry entry;
}

bool _isDayOff(CalendarMark mark) =>
    mark == CalendarMark.holiday || mark == CalendarMark.noClasses;

/// Which day of a run of days off names it: a named holiday, then a named
/// no-class day ("5th Annual Convocation"), then a plain holiday.
int _dayOffRank(CalendarEntry entry) {
  final holiday = markOf(entry) == CalendarMark.holiday;
  final named = switch (titleOf(entry)) {
    'Holiday' || 'No instructional day' => false,
    _ => true,
  };
  return switch ((named, holiday)) {
    (true, true) => 0,
    (true, false) => 1,
    (false, true) => 2,
    (false, false) => 3,
  };
}

/// Whether classes are held on any day strictly between [a] and [b], so
/// two exam blocks around a class week stay separate rows.
bool _classDayBetween(
  Map<DateTime, List<CalendarEntry>> byDate,
  DateTime a,
  DateTime b,
) {
  for (
    var d = DateTime(a.year, a.month, a.day + 1);
    d.isBefore(b);
    d = DateTime(d.year, d.month, d.day + 1)
  ) {
    final mark = _dayMark(byDate[d]);
    if (mark == CalendarMark.classes ||
        mark == CalendarMark.special ||
        mark == CalendarMark.labFat) {
      return true;
    }
  }
  return false;
}

/// The semester's notable days as rows, in date order: holidays, exams,
/// no-class days and named events. Plain class days and Sundays are left
/// out.
List<_Event> _semesterEvents(Map<DateTime, List<CalendarEntry>> byDate) {
  final days = byDate.keys.toList()..sort();
  final events = <_Event>[];
  for (final day in days) {
    for (final entry in byDate[day]!) {
      final mark = markOf(entry);
      if (mark == CalendarMark.classes) continue;
      // Back-to-back days off (Sunday, a no-class Monday, a festival) are
      // one row, named by its most telling day.
      if (_isDayOff(mark)) {
        final previous = events.reversed.where((e) => e.isDayOff).firstOrNull;
        if (previous != null && day.difference(previous.last).inDays <= 1) {
          previous.last = day;
          final part = previous.parts.last;
          if (titleOf(part.entry) == titleOf(entry) &&
              markOf(part.entry) == mark) {
            part.last = day;
          } else {
            previous.parts.add(_Part(day, entry));
          }
          if (_dayOffRank(entry) < _dayOffRank(previous.entry)) {
            previous.entry = entry;
          }
        } else {
          events.add(_Event(day, entry));
        }
        continue;
      }
      // The latest row of the same kind, even with a holiday row after it.
      _Event? previous;
      for (final e in events.reversed) {
        if (markOf(e.entry) == mark && titleOf(e.entry) == titleOf(entry)) {
          previous = e;
          break;
        }
      }
      if (previous != null && !_classDayBetween(byDate, previous.last, day)) {
        previous.last = day;
      } else {
        events.add(_Event(day, entry));
      }
    }
  }
  return events;
}

/// The rows touching the shown month, a run from a neighbouring month
/// included whole.
class _MonthEvents extends StatelessWidget {
  const _MonthEvents({
    super.key,
    required this.month,
    required this.events,
    required this.today,
  });

  final DateTime month;
  final List<_Event> events;
  final DateTime today;

  @override
  Widget build(BuildContext context) {
    final next = DateTime(month.year, month.month + 1);
    final shown = events
        .where((e) => !e.last.isBefore(month) && e.first.isBefore(next))
        .toList();

    if (shown.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: Space.lg),
        child: Text(
          'Regular classes all month.',
          textAlign: TextAlign.center,
          style: context.theme.typography.body.sm.copyWith(
            color: context.theme.colors.mutedForeground,
          ),
        ),
      );
    }

    return Surface(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          for (final (i, event) in shown.indexed) ...[
            if (i > 0)
              Container(
                height: 1,
                margin: const EdgeInsets.only(left: 64),
                color: context.theme.colors.border,
              ),
            _EventRow(event: event, month: month, today: today),
          ],
        ],
      ),
    );
  }
}

class _EventRow extends StatelessWidget {
  const _EventRow({
    required this.event,
    required this.month,
    required this.today,
  });

  final _Event event;
  final DateTime month;
  final DateTime today;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final app = colors.app;
    // A run of days off with any holiday in it reads as a holiday.
    final mark =
        event.isDayOff &&
            event.parts.any((p) => markOf(p.entry) == CalendarMark.holiday)
        ? CalendarMark.holiday
        : markOf(event.entry);
    final past = event.last.isBefore(today);
    final range = event.last != event.first;

    final (label, tone) = switch (mark) {
      CalendarMark.holiday => ('HOLIDAY', app.success),
      CalendarMark.exam => ('EXAM', app.warning),
      CalendarMark.labFat => ('LAB FAT', app.lab),
      CalendarMark.special => ('EVENT', app.accentTone),
      _ => (
        'NO CLASSES',
        Tone(
          base: colors.mutedForeground,
          subtle: colors.secondary,
          onSubtle: colors.mutedForeground,
        ),
      ),
    };
    // Days off for different reasons: one row, each reason as a card.
    final mixed = event.isDayOff && event.parts.length > 1;
    final title = mixed ? '${event.days} days off' : titleOf(event.entry);
    final subtitle = [
      if (range)
        '${DateFormat(event.first.month == event.last.month ? 'EEE d' : 'EEE d MMM').format(event.first)} – '
            '${DateFormat('EEE d MMM').format(event.last)}',
      if (event.isDayOff && range && !mixed) '${event.days} days',
    ].join(' · ');

    return Opacity(
      opacity: past ? 0.5 : 1,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: Space.md,
          vertical: Space.md,
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                SizedBox(
                  width: 40,
                  child: Column(
                    children: [
                      Text(
                        // A run begun last month shows that month, not a weekday.
                        DateFormat(
                          event.first.month == month.month ? 'EEE' : 'MMM',
                        ).format(event.first).toUpperCase(),
                        style: typography.body.xs.copyWith(
                          fontSize: 10.5,
                          fontWeight: FontWeight.w600,
                          letterSpacing: 0.4,
                          color: colors.mutedForeground,
                        ),
                      ),
                      Text(
                        '${event.first.day}',
                        style: typography.body.lg.copyWith(
                          height: 1.15,
                          fontWeight: FontWeight.w600,
                          color: colors.foreground,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: Space.md),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: typography.body.sm.copyWith(
                          fontWeight: FontWeight.w500,
                          color: colors.foreground,
                        ),
                      ),
                      if (subtitle.isNotEmpty)
                        Padding(
                          padding: const EdgeInsets.only(top: 2),
                          child: Text(
                            subtitle,
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
                ),
                const SizedBox(width: Space.sm),
                ToneBadge(label: label, tone: tone),
              ],
            ),
            if (mixed)
              Padding(
                padding: const EdgeInsets.only(top: Space.sm),
                child: Column(
                  children: [
                    for (final part in event.parts)
                      Padding(
                        padding: const EdgeInsets.only(top: Space.xs),
                        child: _PartCard(part: part),
                      ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// One reason inside a run of days off: its days, a mark dot and the name.
class _PartCard extends StatelessWidget {
  const _PartCard({required this.part});

  final _Part part;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final days = part.last == part.first
        ? DateFormat('EEE d').format(part.first)
        : '${DateFormat('EEE d').format(part.first)} – '
              '${DateFormat('EEE d').format(part.last)}';
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(
        horizontal: Space.md,
        vertical: Space.sm,
      ),
      decoration: BoxDecoration(
        color: colors.secondary,
        borderRadius: BorderRadius.circular(Radii.sm),
      ),
      child: Row(
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              color: _markColor(context, markOf(part.entry)),
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: Space.sm),
          SizedBox(
            width: 120,
            child: Text(
              days,
              style: typography.body.xs.copyWith(
                fontWeight: FontWeight.w600,
                color: colors.mutedForeground,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ),
          Expanded(
            child: Text(
              titleOf(part.entry),
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: typography.body.sm.copyWith(color: colors.foreground),
            ),
          ),
        ],
      ),
    );
  }
}
