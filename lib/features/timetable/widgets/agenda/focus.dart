// The focus card at the top of a day, and its pieces.
part of '../agenda_timetable_view.dart';

/// The card at the top: what's happening now and what's next, or a summary
/// when the day hasn't started / is over / is free.
class _FocusPanel extends StatelessWidget {
  const _FocusPanel({
    required this.current,
    required this.next,
    required this.now,
    required this.isToday,
    required this.slots,
    required this.attendance,
    required this.upcomingDay,
    required this.calendarListed,
    this.papers = const [],
    this.nextPaper,
    this.dayOff,
  });

  final TimetableSlot? current;
  final TimetableSlot? next;
  final DateTime now;
  final bool isToday;
  final List<TimetableSlot> slots;
  final List<AttendanceRecord> attendance;
  final ({DateTime date, TimetableSlot first})? upcomingDay;

  /// The calendar entry that makes this a day without classes.
  final CalendarEntry? dayOff;

  /// Whether the academic calendar lists the day. A class-free day it
  /// doesn't list is taken as a holiday.
  final bool calendarListed;

  /// Your exam papers on the day, in time order. They outrank what the
  /// calendar says: VTOP can schedule a paper on a listed holiday.
  final List<ExamPaper> papers;

  /// Your next paper after the day, when it comes before the next class.
  final ExamPaper? nextPaper;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final (current, next) = (this.current, this.next);

    // Day is over, or there's nothing today: point at the next class day.
    if (current == null && next == null) {
      final upcoming = upcomingDay;
      final off = dayOff;
      final unlisted = off == null && slots.isEmpty && !calendarListed;
      final exam = slots.isEmpty ? papers.firstOrNull : null;
      final offMark = exam != null
          ? CalendarMark.exam
          : off == null
          ? (unlisted ? CalendarMark.holiday : null)
          : markOf(off);
      final (icon, tone) = switch (offMark) {
        CalendarMark.exam => (FLucideIcons.penLine, colors.app.warning),
        CalendarMark.holiday => (FLucideIcons.partyPopper, colors.app.success),
        CalendarMark() => (FLucideIcons.calendarOff, colors.app.accentTone),
        null when slots.isEmpty => (
          FLucideIcons.sunMedium,
          colors.app.accentTone,
        ),
        null => (FLucideIcons.circleCheckBig, colors.app.success),
      };
      final typography = context.theme.typography;
      final title = exam != null
          ? '${exam.exam.courseName.trim()}'
                '${papers.length > 1 ? ' +${papers.length - 1} more' : ''}'
          : off != null
          ? titleOf(off)
          : unlisted
          ? 'Holiday'
          : slots.isEmpty
          // The calendar holds classes, you just have none today.
          ? 'Free day'
          : "You're done for today";
      final eyebrow = switch (offMark) {
        // A plain "Holiday" needs no badge repeating it.
        CalendarMark.holiday when title == 'Holiday' => null,
        CalendarMark.exam when exam != null =>
          '${exam.name} · ${formatClock(exam.exam.examTime.split('-').first, context)}',
        CalendarMark.exam => 'EXAM · NO CLASSES',
        CalendarMark.holiday => 'HOLIDAY',
        CalendarMark() => 'NO CLASSES',
        null => null,
      };
      List<Widget> nextLines(String when, String what) => [
        const SizedBox(height: Space.xs),
        Text(
          when,
          maxLines: 1,
          overflow: .ellipsis,
          style: typography.body.sm.nums(
            fontWeight: .w500,
            color: colors.foreground,
          ),
        ),
        const SizedBox(height: 2),
        Text(
          what,
          maxLines: 1,
          overflow: .ellipsis,
          style: typography.body.xs.copyWith(color: colors.mutedForeground),
        ),
      ];
      return Surface(
        child: Row(
          crossAxisAlignment: .start,
          children: [
            _IconTile(icon: icon, tone: tone),
            const SizedBox(width: Space.md),
            Expanded(
              child: Column(
                crossAxisAlignment: .start,
                children: [
                  if (eyebrow != null) ...[
                    ToneBadge(label: eyebrow, tone: tone),
                    const SizedBox(height: Space.xs + 2),
                  ],
                  Text(
                    title,
                    maxLines: 2,
                    overflow: .ellipsis,
                    style: typography.body.md.copyWith(
                      height: 1.25,
                      fontWeight: .w700,
                      color: colors.foreground,
                    ),
                  ),
                  if (nextPaper case final paper?)
                    // An exam comes before any class: "Next: CAT-II ·
                    // Tomorrow 2:00 PM".
                    ...nextLines(
                      'Next: ${paper.name} · ${_dayLabel(paper.start, now)} '
                      '${formatClock(paper.exam.examTime.split('-').first, context)}',
                      paper.exam.courseName.trim(),
                    )
                  else if (upcoming != null)
                    // When: "Back Tue 6 Oct · 11:01 AM" / "Tomorrow · 8:00 AM".
                    ...nextLines(
                      '${off != null || exam != null || unlisted ? 'Back ' : ''}'
                      '${_dayLabel(upcoming.date, now)} · '
                      '${to12H(upcoming.first.startTime, context)}',
                      upcoming.first.name,
                    ),
                ],
              ),
            ),
          ],
        ),
      );
    }

    return Surface(
      padding: .zero,
      borderColor: current != null
          ? colors.app.accent.withValues(alpha: 0.45)
          : null,
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          if (current != null)
            _FocusClass(
              slot: current,
              badge: ToneBadge(
                label: 'NOW',
                tone: colors.app.accentTone,
                solid: true,
              ),
              countdown: _Countdown(
                target: _at(now, current.endTime),
                prefix: 'Ends in',
              ),
              progress:
                  (_timeOfDay(now) - minutesOf(current.startTime)) /
                  (minutesOf(current.endTime) - minutesOf(current.startTime)),
              record: attendanceForSlot(attendance, current),
            ),
          if (current != null && next != null) ...[
            Container(height: 1, color: colors.border),
            _NextStrip(slot: next, now: now),
          ],
          if (current == null && next != null)
            _FocusClass(
              slot: next,
              badge: ToneBadge(
                label: isToday ? 'NEXT' : 'FIRST CLASS',
                tone: colors.app.accentTone,
              ),
              countdown: isToday
                  ? _Countdown(
                      target: _at(now, next.startTime),
                      prefix: 'Starts in',
                    )
                  : null,
              record: attendanceForSlot(attendance, next),
            ),
        ],
      ),
    );
  }
}

class _FocusClass extends StatelessWidget {
  const _FocusClass({
    required this.slot,
    required this.badge,
    required this.record,
    this.countdown,
    this.progress,
  });

  final TimetableSlot slot;
  final Widget badge;
  final Widget? countdown;
  final double? progress;
  final AttendanceRecord? record;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Padding(
      padding: const .all(Space.lg),
      child: Column(
        crossAxisAlignment: .start,
        children: [
          Row(
            children: [
              badge,
              const SizedBox(width: Space.sm),
              if (countdown case final countdown?)
                Expanded(child: countdown)
              else
                const Spacer(),
              Text(
                '${to12H(slot.startTime, context)} – ${to12H(slot.endTime, context)}',
                // A notch under the countdown so both fit on one line.
                style: typography.body.xs.nums(
                  fontSize: (typography.body.xs.fontSize ?? 12) - 2,
                  fontWeight: .w600,
                  color: colors.mutedForeground,
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.md),
          Text(
            slot.name,
            maxLines: 2,
            overflow: .ellipsis,
            style: typography.body.xl.copyWith(
              height: 1.2,
              fontWeight: .w700,
              letterSpacing: -0.2,
              color: colors.foreground,
            ),
          ),
          const SizedBox(height: Space.md),
          // Details on the left, the room as a big tile on the right — it's
          // what you look for on the way to class.
          Row(
            crossAxisAlignment: .end,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: .start,
                  children: [
                    if (slot.faculty.trim().isNotEmpty) ...[
                      _MetaLine(
                        icon: FLucideIcons.user,
                        text: facultyName(slot.faculty),
                      ),
                      const SizedBox(height: Space.sm),
                    ],
                    Row(
                      children: [
                        CourseKindBadge(isLab: slot.kind == ClassKind.lab),
                        const SizedBox(width: Space.sm),
                        Expanded(
                          child: Text(
                            slot.courseCode,
                            maxLines: 1,
                            overflow: .ellipsis,
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                            ),
                          ),
                        ),
                      ],
                    ),
                    if (record != null) ...[
                      const SizedBox(height: Space.sm),
                      _Attendance(record: record!),
                    ],
                  ],
                ),
              ),
              const SizedBox(width: Space.md),
              _RoomTile(slot: slot),
            ],
          ),
          if (progress != null) ...[
            const SizedBox(height: Space.md),
            ProgressBar(value: progress!, color: colors.app.accent, height: 4),
          ],
        ],
      ),
    );
  }
}

/// The room number, large, with its block underneath.
class _RoomTile extends StatelessWidget {
  const _RoomTile({required this.slot});

  final TimetableSlot slot;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Semantics(
      label: 'Room ${slot.roomNo}, ${slot.block}',
      excludeSemantics: true,
      child: Container(
        width: 88,
        padding: const .symmetric(horizontal: Space.sm, vertical: Space.sm + 2),
        decoration: BoxDecoration(
          color: colors.secondary,
          borderRadius: .circular(Radii.md),
          border: .all(color: colors.border),
        ),
        child: Column(
          children: [
            Text(
              'ROOM',
              style: TextStyle(
                fontSize: 9,
                letterSpacing: 0.8,
                fontWeight: .w600,
                color: colors.mutedForeground,
              ),
            ),
            FittedBox(
              fit: .scaleDown,
              child: Text(
                slot.roomNo,
                maxLines: 1,
                style: numStyle(
                  fontSize: 26,
                  height: 1.15,
                  fontWeight: .w800,
                  letterSpacing: -0.5,
                  color: colors.foreground,
                ),
              ),
            ),
            Text(
              slot.block,
              maxLines: 1,
              overflow: .ellipsis,
              style: TextStyle(
                fontSize: 10.5,
                fontWeight: .w600,
                color: colors.mutedForeground,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Compact "up next" line under the current class.
class _NextStrip extends StatelessWidget {
  const _NextStrip({required this.slot, required this.now});

  final TimetableSlot slot;
  final DateTime now;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final gap = minutesOf(slot.startTime) - _timeOfDay(now);
    return Padding(
      padding: const .fromLTRB(Space.lg, Space.md, Space.lg, Space.md),
      child: Row(
        children: [
          Text(
            'NEXT',
            style: typography.body.xs.copyWith(
              fontWeight: .w700,
              letterSpacing: 0.6,
              color: colors.app.accentTone.onSubtle,
            ),
          ),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: .start,
              children: [
                Text(
                  slot.name,
                  maxLines: 1,
                  overflow: .ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: .w600,
                    color: colors.foreground,
                  ),
                ),
                Text(
                  '${to12H(slot.startTime, context)} · Room ${slot.roomNo}',
                  maxLines: 1,
                  overflow: .ellipsis,
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.sm),
          Text(
            gap <= 0 ? 'now' : 'in ${formatMinutes(gap)}',
            style: typography.body.xs.copyWith(
              fontWeight: .w600,
              color: colors.mutedForeground,
            ),
          ),
        ],
      ),
    );
  }
}

/// Live h:mm:ss countdown to [target]; ticks once a second on its own so the
/// rest of the view doesn't rebuild.
class _Countdown extends HookWidget {
  const _Countdown({required this.target, required this.prefix});

  final DateTime target;
  final String prefix;

  @override
  Widget build(BuildContext context) {
    final now = useState(DateTime.now());
    useEffect(() {
      final timer = Timer.periodic(
        const Duration(seconds: 1),
        (_) => now.value = DateTime.now(),
      );
      return timer.cancel;
    }, const []);
    final left = target.difference(now.value);
    final seconds = left.isNegative ? 0 : left.inSeconds;
    final h = seconds ~/ 3600;
    final m = (seconds % 3600) ~/ 60;
    final s = seconds % 60;
    String two(int v) => v.toString().padLeft(2, '0');
    final value = h > 0 ? '$h:${two(m)}:${two(s)}' : '$m:${two(s)}';

    return Semantics(
      liveRegion: false,
      label: '$prefix ${h > 0 ? '$h hours ' : ''}$m minutes',
      child: ExcludeSemantics(
        child: Text.rich(
          TextSpan(
            children: [
              TextSpan(text: '$prefix '),
              TextSpan(
                text: value,
                style: TextStyle(
                  fontWeight: .w700,
                  color: context.theme.colors.foreground,
                ),
              ),
            ],
          ),
          maxLines: 1,
          overflow: .ellipsis,
          style: context.theme.typography.body.xs.nums(
            color: context.theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}
