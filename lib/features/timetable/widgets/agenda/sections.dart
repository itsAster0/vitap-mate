// Below the day: what comes up, the week recap, the lookahead and exam rows.
part of '../agenda_timetable_view.dart';

/// What follows a day without classes. Normally the next class day, with
/// each class's attendance so you know what you can't miss; when exams come
/// before any class (CAT or FAT week), the next papers instead. The corner
/// shows the next exam or holiday within [_soon], else class days left.
class _ComingUp extends StatelessWidget {
  const _ComingUp({
    required this.from,
    required this.now,
    required this.nextDay,
    required this.slotsOn,
    required this.exams,
    required this.attendance,
    required this.calendar,
  });

  /// The day being shown.
  final DateTime from;
  final DateTime now;
  final ({DateTime date, TimetableSlot first})? nextDay;
  final List<TimetableSlot> Function(DateTime date) slotsOn;
  final List<ExamPaper> exams;
  final List<AttendanceRecord> attendance;
  final SemesterCalendar? calendar;

  static const _soon = 14;
  static const _maxPapers = 3;

  @override
  Widget build(BuildContext context) {
    final nextDate = nextDay?.date;
    // Papers still to be written, from the shown day up to the next class.
    final papers = _papersBetween(
      exams,
      from: from,
      now: now,
      before: nextDate,
    );

    if (papers.isNotEmpty) {
      return Column(
        crossAxisAlignment: .stretch,
        children: [
          SectionHeader(
            title: 'Coming up',
            trailing: Text(
              '${papers.length} ${papers.length == 1 ? 'paper' : 'papers'} left',
            ),
          ),
          Surface(
            child: Column(
              crossAxisAlignment: .stretch,
              children: [
                for (final (i, p) in papers.take(_maxPapers).indexed) ...[
                  if (i > 0) const SizedBox(height: Space.md),
                  _PaperRow(start: p.start, exam: p.exam, now: now),
                ],
                if (papers.length > _maxPapers) ...[
                  const SizedBox(height: Space.md),
                  Text(
                    '+${papers.length - _maxPapers} more in Exam Schedule',
                    style: context.theme.typography.body.xs.copyWith(
                      color: context.theme.colors.mutedForeground,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ],
      );
    }

    if (nextDate == null) return const SizedBox.shrink();
    final slots = slotsOn(nextDate);
    if (slots.isEmpty) return const SizedBox.shrink();
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final corner = _corner();
    return Column(
      crossAxisAlignment: .stretch,
      children: [
        SectionHeader(
          title: 'Coming up',
          trailing: corner == null ? null : Text(corner),
        ),
        Surface(
          child: Column(
            crossAxisAlignment: .stretch,
            children: [
              Text(
                '${_dayLabel(nextDate, now)}  ·  ${slots.length} '
                '${slots.length == 1 ? 'class' : 'classes'}  ·  '
                '${to12H(slots.first.startTime, context)} – '
                '${to12H(slots.last.endTime, context)}',
                maxLines: 1,
                overflow: .ellipsis,
                style: typography.body.sm.nums(
                  fontWeight: .w600,
                  color: colors.foreground,
                ),
              ),
              const SizedBox(height: Space.sm + 2),
              _DayBar(slots: slots, minuteNow: null),
              const SizedBox(height: Space.sm),
              for (final slot in slots)
                Padding(
                  padding: const .only(top: Space.sm),
                  child: Row(
                    children: [
                      SizedBox(
                        width: 76,
                        child: Text(
                          to12H(slot.startTime, context),
                          maxLines: 1,
                          softWrap: false,
                          style: typography.body.xs.nums(
                            color: colors.mutedForeground,
                          ),
                        ),
                      ),
                      Expanded(
                        child: Text.rich(
                          TextSpan(
                            children: [
                              TextSpan(text: slot.name),
                              if (slot.kind == ClassKind.lab)
                                TextSpan(
                                  text: '  Lab',
                                  style: TextStyle(
                                    color: colors.mutedForeground,
                                  ),
                                ),
                            ],
                          ),
                          maxLines: 1,
                          overflow: .ellipsis,
                          style: typography.body.sm.copyWith(
                            color: colors.foreground,
                          ),
                        ),
                      ),
                      if (attendanceForSlot(attendance, slot)
                          case final record?) ...[
                        const SizedBox(width: Space.sm),
                        _Attendance(record: record),
                      ],
                    ],
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }

  /// "CAT-II in 2 days", "Deepavali in 5 days", or "38 class days left".
  String? _corner() {
    final calendar = this.calendar;
    if (calendar == null) return null;
    final today = DateTime(now.year, now.month, now.day);
    String inDays(DateTime date) => _inDaysLabel(date.difference(today).inDays);

    final exam = calendar.upcomingExam(now);
    if (exam != null && exam.start.difference(today).inDays <= _soon) {
      return '${exam.name} ${inDays(exam.start)}';
    }
    final holiday = calendar.nextHoliday(now);
    if (holiday != null &&
        holiday.date.isAfter(today) &&
        holiday.date.difference(today).inDays <= _soon) {
      // Long festival names would crowd the header.
      final name = holiday.name.length > 16 ? 'Holiday' : holiday.name;
      return '$name ${inDays(holiday.date)}';
    }
    final left = calendar.instructionalDaysLeft(now);
    return left > 0 ? '$left class days left' : null;
  }
}

/// A week's attendance, for a day you have no classes: one dot per class on
/// each day that had any, coloured by what the history says (present, on
/// duty, absent), hollow when held but not posted yet, faint when still to
/// come.
class _WeekRecap extends StatelessWidget {
  const _WeekRecap({
    required this.thisWeek,
    required this.dates,
    required this.now,
    required this.classesOn,
    required this.history,
  });

  /// Whether [dates] are the current week (a Sunday) rather than last week.
  final bool thisWeek;

  /// Monday to Sunday of the recapped week.
  final List<DateTime> dates;
  final DateTime now;

  /// The classes that meet on a date.
  final List<TimetableSlot> Function(DateTime date) classesOn;
  final Map<(String, bool), FullAttendanceData> history;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final palette = colors.app;
    final today = DateTime(now.year, now.month, now.day);
    final minuteNow = _timeOfDay(now);

    // Per course: its rows by day, and the last day it has a row for.
    final rowsByCourse = <(String, bool), Map<DateTime, List<ClassStatus>>>{};
    final lastRecorded = <(String, bool), DateTime>{};
    for (final MapEntry(key: course, value: data) in history.entries) {
      final byDay = rowsByCourse[course] = {};
      for (final row in data.records) {
        final date = parseHistoryDate(row.date);
        if (date == null) continue;
        byDay.putIfAbsent(date, () => []).add(classStatusOf(row.status));
        final last = lastRecorded[course];
        if (last == null || date.isAfter(last)) lastRecorded[course] = date;
      }
    }

    // Per day: posted statuses, held classes VTOP hasn't posted yet (after
    // that course's last row), and classes still to come. A held class
    // missing from a history that has moved past it was never recorded
    // (likely cancelled), so it is left out.
    var total = 0;
    var unposted = 0;
    final days = [
      for (final date in dates)
        () {
          final perCourse = <(String, bool), List<TimetableSlot>>{};
          for (final slot in classesOn(date)) {
            perCourse
                .putIfAbsent((
                  slot.courseCode,
                  slot.kind == ClassKind.lab,
                ), () => [])
                .add(slot);
          }
          final statuses = <ClassStatus>[];
          var missing = 0;
          var upcoming = 0;
          for (final MapEntry(key: course, value: slots) in perCourse.entries) {
            final rows = rowsByCourse[course]?[date] ?? const <ClassStatus>[];
            statuses.addAll(rows);
            final held = date.isBefore(today)
                ? slots.length
                : date == today
                ? slots.where((s) => minutesOf(s.startTime) <= minuteNow).length
                : 0;
            upcoming += slots.length - held;
            final last = lastRecorded[course];
            if (held > rows.length &&
                history.containsKey(course) &&
                (last == null || date.isAfter(last))) {
              missing += held - rows.length;
            }
          }
          // Rows for classes the timetable doesn't list that day (extra
          // classes) still count.
          for (final MapEntry(key: course, value: byDay)
              in rowsByCourse.entries) {
            if (!perCourse.containsKey(course)) {
              statuses.addAll(byDay[date] ?? const []);
            }
          }
          statuses.sort((a, b) => a.index.compareTo(b.index));
          total += statuses.length + missing + upcoming;
          unposted += missing;
          return (
            date: date,
            statuses: statuses,
            unposted: missing,
            upcoming: upcoming,
          );
        }(),
    ];
    if (total == 0) return const SizedBox.shrink();

    int count(ClassStatus s) =>
        days.fold(0, (n, d) => n + d.statuses.where((x) => x == s).length);
    final present = count(ClassStatus.present);
    final onDuty = count(ClassStatus.onDuty);
    final absent = count(ClassStatus.absent);
    // (count, label, colour) for the centred summary line.
    final summary = [
      if (present > 0) (present, 'present', palette.success.onSubtle),
      if (onDuty > 0) (onDuty, 'on duty', palette.accentTone.onSubtle),
      if (absent > 0) (absent, 'absent', palette.danger.onSubtle),
      if (unposted > 0) (unposted, 'not posted', colors.foreground),
    ];

    Color colorOf(ClassStatus s) => switch (s) {
      ClassStatus.present => palette.success.base,
      ClassStatus.onDuty => palette.accent,
      ClassStatus.absent => palette.danger.base,
      ClassStatus.other => colors.mutedForeground,
    };
    Widget dot({Color? fill, Color? ring}) =>
        Dot(size: 6, color: fill, ring: ring);

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        SectionHeader(
          title: thisWeek ? 'This week' : 'Last week',
          // "14–20 Sep · 22 classes".
          trailing: Text(
            '${dates.first.day}–${DateFormat('d MMM').format(dates.last)}'
            '  ·  $total ${total == 1 ? 'class' : 'classes'}',
          ),
        ),
        Surface(
          child: Column(
            crossAxisAlignment: .stretch,
            children: [
              Row(
                crossAxisAlignment: .start,
                children: [
                  // Only days that had classes; holidays, no-instructional
                  // days and Sundays are left out.
                  for (final day in days.where(
                    (d) => d.statuses.isNotEmpty || d.unposted + d.upcoming > 0,
                  ))
                    Expanded(
                      child: Semantics(
                        label:
                            '${DateFormat('EEEE').format(day.date)}, '
                            '${day.statuses.length + day.unposted + day.upcoming} classes',
                        child: Column(
                          children: [
                            Text(
                              DateFormat('EEEEE').format(day.date),
                              style: typography.body.xs.copyWith(
                                fontSize: 10,
                                fontWeight: .w600,
                                color: colors.mutedForeground,
                              ),
                            ),
                            Text(
                              '${day.date.day}',
                              style: typography.body.sm.nums(
                                fontWeight: .w600,
                                color: colors.foreground,
                              ),
                            ),
                            const SizedBox(height: Space.sm),
                            // Two columns of dots keep a six-class day short.
                            Wrap(
                              spacing: 3,
                              runSpacing: 3,
                              alignment: .center,
                              children: [
                                for (final s in day.statuses)
                                  dot(fill: colorOf(s)),
                                for (var i = 0; i < day.unposted; i++)
                                  dot(ring: colors.mutedForeground),
                                for (var i = 0; i < day.upcoming; i++)
                                  dot(fill: colors.border),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                ],
              ),
              if (summary.isNotEmpty) ...[
                const SizedBox(height: Space.md),
                Container(height: 1, color: colors.border),
                const SizedBox(height: Space.md),
                // A two-column grid, each count centred in its half; a lone
                // count on the last row sits in the middle.
                for (var row = 0; row < summary.length; row += 2) ...[
                  if (row > 0) const SizedBox(height: Space.sm),
                  Row(
                    children: [
                      for (
                        var col = row;
                        col < row + 2 && col < summary.length;
                        col++
                      )
                        Expanded(
                          child: Text.rich(
                            TextSpan(
                              children: [
                                TextSpan(
                                  text: '${summary[col].$1}',
                                  style: TextStyle(
                                    fontWeight: .w700,
                                    color: summary[col].$3,
                                  ),
                                ),
                                TextSpan(text: ' ${summary[col].$2}'),
                              ],
                            ),
                            textAlign: .center,
                            style: typography.body.sm.nums(
                              color: colors.mutedForeground,
                            ),
                          ),
                        ),
                    ],
                  ),
                ],
              ],
            ],
          ),
        ),
      ],
    );
  }
}

/// Sunday's look at the next two weeks: exams (grouped into their run of
/// days) and every holiday or no-class day, from the academic calendar.
class _Lookahead extends StatelessWidget {
  const _Lookahead({
    required this.from,
    required this.calendar,
    this.papers = const [],
    this.hide = const {},
  });

  final DateTime from;
  final SemesterCalendar calendar;

  /// Your exam papers. A day with one is an exam day whatever the calendar
  /// says (VTOP can set a paper on a listed holiday).
  final List<ExamPaper> papers;

  /// Exam names to leave out, already listed paper by paper above.
  final Set<String> hide;

  static const _days = 14;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final palette = colors.app;
    final end = DateTime(from.year, from.month, from.day + _days);

    // Exam runs by name (CAT-II can skip a holiday and a Sunday), and every
    // day off (Sundays included) on its own.
    final runs = <String, ({DateTime first, DateTime last, int days})>{};
    final events =
        <
          ({
            DateTime start,
            DateTime? last,
            String title,
            String detail,
            Tone tone,
          })
        >[];
    for (var i = 1; i <= _days; i++) {
      final date = DateTime(from.year, from.month, from.day + i);
      final paper = papers.where((p) => _sameDay(p.start, date)).firstOrNull;
      final entry = calendar.entryOn(date);
      if (entry == null && paper == null) continue;
      final mark = paper != null ? CalendarMark.exam : markOf(entry!);
      switch (mark) {
        case CalendarMark.exam || CalendarMark.labFat:
          final name = paper?.name ?? titleOf(entry!);
          final run = runs[name];
          runs[name] = run == null
              ? (first: date, last: date, days: 1)
              : (first: run.first, last: date, days: run.days + 1);
        case CalendarMark.holiday || CalendarMark.noClasses:
          events.add((
            start: date,
            last: null,
            title: titleOf(entry!),
            // A plain "Holiday" (most Sundays) names the day instead of
            // repeating itself.
            detail: titleOf(entry) == 'Holiday'
                ? DateFormat('EEEE').format(date)
                : mark == CalendarMark.holiday
                ? 'Holiday'
                : 'No classes',
            tone: mark == CalendarMark.holiday
                ? palette.success
                : palette.accentTone,
          ));
        default:
          break;
      }
    }
    // Mark the end of classes when it falls in these two weeks.
    final lastClass = calendar.lastClassDay(lab: false);
    if (lastClass != null &&
        lastClass.isAfter(from) &&
        !lastClass.isAfter(end)) {
      events.add((
        start: lastClass,
        last: null,
        title: 'Last class day',
        detail: 'Classes end',
        tone: palette.accentTone,
      ));
    }
    // Past the last class day the calendar says little; don't claim
    // classes as usual.
    final over = lastClass == null || !lastClass.isAfter(from);
    for (final MapEntry(key: name, value: run) in runs.entries) {
      if (hide.contains(name)) continue;
      events.add((
        start: run.first,
        last: run.days > 1 ? run.last : null,
        title: name,
        detail: '${run.days} exam ${run.days == 1 ? 'day' : 'days'}',
        tone: palette.warning,
      ));
    }
    events.sort((a, b) => a.start.compareTo(b.start));

    String inDays(DateTime date) => _inDaysLabel(date.difference(from).inDays);

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        SectionHeader(
          title: 'Next two weeks',
          trailing: Text(
            '${DateFormat('d MMM').format(from.add(const Duration(days: 1)))}'
            ' – ${DateFormat('d MMM').format(end)}',
          ),
        ),
        Surface(
          child: events.isEmpty
              ? Text(
                  over
                      ? 'No more classes this semester.'
                      : 'No exams or holidays. Classes as usual.',
                  style: typography.body.sm.copyWith(
                    color: colors.mutedForeground,
                  ),
                )
              : Column(
                  crossAxisAlignment: .stretch,
                  children: [
                    for (final (i, e) in events.indexed) ...[
                      if (i > 0) const SizedBox(height: Space.md),
                      _DatedRow(
                        date: e.start,
                        fill: e.tone.subtle,
                        weekday: e.tone.onSubtle,
                        number: e.tone.onSubtle,
                        title: e.title,
                        detail: [
                          if (e.last case final last?)
                            'Till ${DateFormat('EEE d MMM').format(last)}',
                          e.detail,
                          inDays(e.start),
                        ].join('  ·  '),
                      ),
                    ],
                  ],
                ),
        ),
      ],
    );
  }
}

/// One exam paper: its day, the course, and time and venue.
class _PaperRow extends StatelessWidget {
  const _PaperRow({required this.start, required this.exam, required this.now});

  final DateTime start;
  final ExamScheduleRecord exam;
  final DateTime now;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final today = DateTime(now.year, now.month, now.day);
    final isToday = DateTime(start.year, start.month, start.day) == today;
    final details = [
      if (hasValue(exam.examTime))
        formatClock(exam.examTime.split('-').first, context),
      if (hasValue(exam.venue)) exam.venue.trim(),
      if (hasValue(exam.seatNo)) 'Seat ${exam.seatNo.trim()}',
    ].join('  ·  ');
    final tint = isToday ? colors.app.warning.onSubtle : null;
    return _DatedRow(
      date: start,
      fill: isToday ? colors.app.warning.subtle : colors.secondary,
      weekday: tint ?? colors.mutedForeground,
      number: tint ?? colors.foreground,
      title: exam.courseName.trim(),
      detail: details.isEmpty ? null : details,
      tabular: true,
    );
  }
}

/// "tomorrow" or "in 5 days".
String _inDaysLabel(int days) => days == 1 ? 'tomorrow' : 'in $days days';

/// A day tile (weekday over its number) beside a title and a detail line.
class _DatedRow extends StatelessWidget {
  const _DatedRow({
    required this.date,
    required this.fill,
    required this.weekday,
    required this.number,
    required this.title,
    required this.detail,
    this.tabular = false,
  });

  final DateTime date;
  final Color fill;

  /// Colours of the tile's weekday and day number.
  final Color weekday;
  final Color number;
  final String title;
  final String? detail;

  /// Tabular figures in [detail].
  final bool tabular;

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
              if (detail case final detail?)
                Text(
                  detail,
                  maxLines: 1,
                  overflow: .ellipsis,
                  style: tabular
                      ? typography.body.xs.nums(color: colors.mutedForeground)
                      : typography.body.xs.copyWith(
                          color: colors.mutedForeground,
                        ),
                ),
            ],
          ),
        ),
      ],
    );
  }
}
