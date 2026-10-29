import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/calendar/logic/calendar_events.dart';
import 'package:vitapmate/features/calendar/widgets/month_grid.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/ui/ui.dart';

/// The rows touching the shown month, a run from a neighbouring month
/// included whole.
class MonthEvents extends StatelessWidget {
  const MonthEvents({
    super.key,
    required this.month,
    required this.events,
    required this.today,
  });

  final DateTime month;
  final List<CalendarEvent> events;
  final DateTime today;

  @override
  Widget build(BuildContext context) {
    final next = DateTime(month.year, month.month + 1);
    final shown = events
        .where((e) => !e.last.isBefore(month) && e.first.isBefore(next))
        .toList();

    if (shown.isEmpty) {
      return Padding(
        padding: const .symmetric(vertical: Space.lg),
        child: Text(
          'Regular classes all month.',
          textAlign: .center,
          style: context.theme.typography.body.sm.copyWith(
            color: context.theme.colors.mutedForeground,
          ),
        ),
      );
    }

    return Surface(
      padding: .zero,
      child: Column(
        children: [
          for (final (i, event) in shown.indexed) ...[
            if (i > 0)
              Container(
                height: 1,
                margin: const .only(left: 64),
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

  final CalendarEvent event;
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
        padding: const .symmetric(horizontal: Space.md, vertical: Space.md),
        child: Column(
          crossAxisAlignment: .start,
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
                          fontWeight: .w600,
                          letterSpacing: 0.4,
                          color: colors.mutedForeground,
                        ),
                      ),
                      Text(
                        '${event.first.day}',
                        style: typography.body.lg.nums(
                          height: 1.15,
                          fontWeight: .w600,
                          color: colors.foreground,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: Space.md),
                Expanded(
                  child: Column(
                    crossAxisAlignment: .start,
                    children: [
                      Text(
                        title,
                        maxLines: 2,
                        overflow: .ellipsis,
                        style: typography.body.sm.copyWith(
                          fontWeight: .w500,
                          color: colors.foreground,
                        ),
                      ),
                      if (subtitle.isNotEmpty)
                        Padding(
                          padding: const .only(top: 2),
                          child: Text(
                            subtitle,
                            style: typography.body.xs.nums(
                              color: colors.mutedForeground,
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
                padding: const .only(top: Space.sm),
                child: Column(
                  children: [
                    for (final part in event.parts)
                      Padding(
                        padding: const .only(top: Space.xs),
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

  final CalendarEventPart part;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final day = DateFormat('EEE d');
    final days = part.last == part.first
        ? day.format(part.first)
        : '${day.format(part.first)} – ${day.format(part.last)}';
    return Container(
      width: double.infinity,
      padding: const .symmetric(horizontal: Space.md, vertical: Space.sm),
      decoration: BoxDecoration(
        color: colors.secondary,
        borderRadius: .circular(Radii.sm),
      ),
      child: Row(
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              color: markColor(context, markOf(part.entry)),
              shape: .circle,
            ),
          ),
          const SizedBox(width: Space.sm),
          SizedBox(
            width: 120,
            child: Text(
              days,
              style: typography.body.xs.nums(
                fontWeight: .w600,
                color: colors.mutedForeground,
              ),
            ),
          ),
          Expanded(
            child: Text(
              titleOf(part.entry),
              maxLines: 2,
              overflow: .ellipsis,
              style: typography.body.sm.copyWith(color: colors.foreground),
            ),
          ),
        ],
      ),
    );
  }
}
