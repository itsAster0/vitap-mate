// Below a day without classes: the week ahead and attendance to watch.
part of '../agenda_timetable_view.dart';

/// The seven days after a free day, one row each: your classes on a shared
/// time scale, or what the calendar calls the day (a run of the same day
/// off on one row). A row in the week strip opens that day.
class _WeekAhead extends StatelessWidget {
  const _WeekAhead({
    required this.from,
    required this.calendar,
    required this.papers,
    required this.slotsOn,
    required this.labFatsOn,
    required this.onOpen,
  });

  final DateTime from;
  final SemesterCalendar? calendar;

  /// Your exam papers. A day with one is an exam day whatever the calendar
  /// says (VTOP can set a paper on a listed holiday).
  final List<ExamPaper> papers;
  final List<TimetableSlot> Function(DateTime date) slotsOn;
  final List<TimetableSlot> Function(DateTime date) labFatsOn;

  /// Opens a date, or null when it isn't in the week strip.
  final VoidCallback? Function(DateTime date) onOpen;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final palette = colors.app;
    final lastClass = calendar?.lastClassDay(lab: false);
    final rows =
        <
          ({
            DateTime date,
            DateTime? till,
            String title,
            String? detail,
            Tone? tone,
            List<TimetableSlot> slots,
          })
        >[];
    for (var i = 1; i <= 7; i++) {
      final date = DateTime(from.year, from.month, from.day + i);
      final dayPapers = [
        for (final p in papers)
          if (_sameDay(p.start, date)) p,
      ];
      final slots = slotsOn(date);
      if (dayPapers.isNotEmpty) {
        final paper = dayPapers.first;
        rows.add((
          date: date,
          till: null,
          title: '${paper.name} · ${paper.exam.courseName.trim()}',
          detail: [
            formatClock(paper.exam.examTime.split('-').first, context),
            if (dayPapers.length > 1) '+${dayPapers.length - 1} more',
          ].join('  ·  '),
          tone: palette.warning,
          slots: const [],
        ));
      } else if (slots.isNotEmpty) {
        final fats = {for (final s in labFatsOn(date)) s.courseCode};
        rows.add((
          date: date,
          till: null,
          title: '${slots.length} ${slots.length == 1 ? 'class' : 'classes'}',
          detail: [
            '${to12H(slots.first.startTime, context)} – '
                '${to12H(slots.last.endTime, context)}',
            if (fats.isNotEmpty) 'Lab FAT ${fats.join(', ')}',
            if (date == lastClass) 'Last class day',
          ].join('  ·  '),
          tone: null,
          slots: slots,
        ));
      } else {
        final entry = calendar?.entryOn(date);
        final mark = entry == null ? null : markOf(entry);
        final named =
            mark == CalendarMark.holiday ||
            mark == CalendarMark.noClasses ||
            mark == CalendarMark.exam;
        final title = named ? titleOf(entry!) : 'No classes';
        // A run of the same day off is one row.
        final previous = rows.lastOrNull;
        if (previous != null &&
            previous.slots.isEmpty &&
            previous.title == title &&
            DateTime(date.year, date.month, date.day - 1) ==
                (previous.till ?? previous.date)) {
          rows.last = (
            date: previous.date,
            till: date,
            title: title,
            detail: previous.detail,
            tone: previous.tone,
            slots: const [],
          );
          continue;
        }
        rows.add((
          date: date,
          till: null,
          title: title,
          detail: mark == CalendarMark.holiday && title != 'Holiday'
              ? 'Holiday'
              : null,
          tone: switch (mark) {
            CalendarMark.holiday => palette.success,
            CalendarMark.exam => palette.warning,
            _ => null,
          },
          slots: const [],
        ));
      }
    }
    // A run of days off that carries on past the week shows its real end.
    if (rows.lastOrNull case final tail? when tail.slots.isEmpty) {
      var till = tail.till ?? tail.date;
      while (true) {
        final next = DateTime(till.year, till.month, till.day + 1);
        final entry = calendar?.entryOn(next);
        if (entry == null ||
            titleOf(entry) != tail.title ||
            slotsOn(next).isNotEmpty) {
          break;
        }
        till = next;
      }
      if (till != tail.date) {
        rows.last = (
          date: tail.date,
          till: till,
          title: tail.title,
          detail: tail.detail,
          tone: tail.tone,
          slots: const [],
        );
      }
    }
    // Every day's bar on the same clock, so the days compare at a glance.
    final classDays = rows.where((r) => r.slots.isNotEmpty);
    final span = classDays.isEmpty
        ? null
        : (
            classDays
                .map((r) => minutesOf(r.slots.first.startTime))
                .reduce(math.min),
            classDays
                .map((r) => minutesOf(r.slots.last.endTime))
                .reduce(math.max),
          );
    final end = DateTime(from.year, from.month, from.day + 7);

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        SectionHeader(
          title: 'Week ahead',
          trailing: Text(
            '${DateFormat('d MMM').format(from.add(const Duration(days: 1)))}'
            ' – ${DateFormat('d MMM').format(end)}',
          ),
        ),
        Surface(
          child: Column(
            crossAxisAlignment: .stretch,
            children: [
              for (final (i, r) in rows.indexed) ...[
                if (i > 0) const SizedBox(height: Space.md),
                Builder(
                  builder: (context) {
                    final row = _DatedRow(
                      date: r.date,
                      fill: r.tone?.subtle ?? colors.secondary,
                      weekday: r.tone?.onSubtle ?? colors.mutedForeground,
                      number: r.tone?.onSubtle ?? colors.foreground,
                      title: r.title,
                      detail: [
                        if (r.till case final till?)
                          'Till ${DateFormat('EEE d MMM').format(till)}',
                        ?r.detail,
                      ].join('  ·  '),
                      below: r.slots.isEmpty
                          ? null
                          : _DayBar(
                              slots: r.slots,
                              minuteNow: null,
                              span: span,
                            ),
                    );
                    final open = onOpen(r.date);
                    return open == null
                        ? row
                        : PressScale(
                            semanticsLabel:
                                'Open ${DateFormat('EEEE').format(r.date)}',
                            onPress: open,
                            child: row,
                          );
                  },
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

/// Courses that can't spare a class, lowest first; one calm line when none.
class _AttendanceWatch extends ConsumerWidget {
  const _AttendanceWatch({required this.records});

  final List<AttendanceRecord> records;

  static const _max = 4;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final skips = ref.watch(showSkipAdviceProvider);
    final all =
        [
          for (final r in records)
            (record: r, standing: AttendanceStanding.sessions(r)),
        ]..sort(
          (a, b) =>
              a.standing.displayPercent.compareTo(b.standing.displayPercent),
        );
    if (all.isEmpty) return const SizedBox.shrink();
    final watch = [
      for (final c in all)
        if (!c.standing.isSafe || (skips && c.standing.canSkip == 0)) c,
    ];
    final lowest = all.first;

    Widget course(
      ({AttendanceRecord record, AttendanceStanding standing}) c, {
      String? title,
    }) {
      final s = c.standing;
      final tone = attendanceTone(
        context,
        safe: s.isSafe,
        atEdge: s.canSkip == 0,
      );
      final parts = c.record.courseName.split(' - ');
      final code = '${courseCodeOf(c.record)}${c.record.islab() ? ' lab' : ''}';
      return Row(
        children: [
          ProgressRing(
            value: s.displayPercent / 100,
            color: tone.base,
            size: 28,
            stroke: 3,
          ),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: .start,
              children: [
                Text(
                  title ?? code,
                  maxLines: 1,
                  overflow: .ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: .w600,
                    color: colors.foreground,
                  ),
                ),
                Text(
                  title != null
                      ? 'Lowest: $code · ${s.displayPercent.round()}%'
                      : parts.length > 1
                      ? parts[1].trim()
                      : c.record.courseName,
                  maxLines: 1,
                  overflow: .ellipsis,
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          if (title == null) ...[
            const SizedBox(width: Space.sm),
            Text(
              [
                '${s.displayPercent.round()}%',
                if (skips) !s.isSafe ? 'need ${s.mustAttend}' : "don't skip",
              ].join(' · '),
              style: typography.body.xs.nums(
                fontWeight: .w600,
                color: tone.onSubtle,
              ),
            ),
          ],
        ],
      );
    }

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        SectionHeader(
          title: 'Attendance',
          trailing: watch.isEmpty ? null : Text('${watch.length} to watch'),
        ),
        Surface(
          child: watch.isEmpty
              ? course(lowest, title: 'All courses above 75%')
              : Column(
                  crossAxisAlignment: .stretch,
                  children: [
                    for (final (i, c) in watch.take(_max).indexed) ...[
                      if (i > 0) const SizedBox(height: Space.md),
                      course(c),
                    ],
                  ],
                ),
        ),
      ],
    );
  }
}

/// A day tile (weekday over its number) beside a title and a detail line.
class _DatedRow extends StatelessWidget {
  const _DatedRow({
    required this.date,
    required this.fill,
    required this.weekday,
    required this.number,
    required this.title,
    required this.detail,
    this.below,
  });

  final DateTime date;
  final Color fill;

  /// Colours of the tile's weekday and day number.
  final Color weekday;
  final Color number;
  final String title;
  final String? detail;

  /// Under the detail line, e.g. the day's classes as a bar.
  final Widget? below;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Row(
      children: [
        Container(
          width: 44,
          padding: const .symmetric(vertical: Space.xs),
          decoration: BoxDecoration(
            color: fill,
            borderRadius: .circular(Radii.md),
          ),
          child: Column(
            children: [
              Text(
                DateFormat('EEE').format(date).toUpperCase(),
                style: typography.body.xs.copyWith(
                  fontSize: 10,
                  fontWeight: .w600,
                  color: weekday,
                ),
              ),
              Text(
                '${date.day}',
                style: typography.body.md.nums(
                  height: 1.1,
                  fontWeight: .w700,
                  color: number,
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
                maxLines: 1,
                overflow: .ellipsis,
                style: typography.body.sm.copyWith(
                  fontWeight: .w600,
                  color: colors.foreground,
                ),
              ),
              if (detail case final detail? when detail.isNotEmpty)
                Text(
                  detail,
                  maxLines: 1,
                  overflow: .ellipsis,
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              if (below case final below?) ...[
                const SizedBox(height: Space.xs),
                below,
              ],
            ],
          ),
        ),
      ],
    );
  }
}
