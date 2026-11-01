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
    required this.labFats,
    required this.isLastClass,
    this.done,
    this.papers = const [],
    this.nextPaper,
    this.dayOff,
    this.countdown,
  });

  final TimetableSlot? current;
  final TimetableSlot? next;
  final DateTime now;
  final bool isToday;
  final List<TimetableSlot> slots;
  final List<AttendanceRecord> attendance;
  final ({DateTime date, TimetableSlot first})? upcomingDay;

  /// The day's Lab FATs: no attendance, an amber chip.
  final Set<TimetableSlot> labFats;
  final bool Function(TimetableSlot slot) isLastClass;

  /// The day's classes with their posted attendance, once they are over.
  final List<({TimetableSlot slot, ClassStatus? status})>? done;

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

  /// Where the semester stands on a free day: "12 class days left · FAT
  /// starts Mon 16 Nov".
  final String? countdown;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final (current, next) = (this.current, this.next);

    // Classes are over: how the day went.
    if (done case final done?) {
      return _DaySummary(classes: done, labFats: labFats, isToday: isToday);
    }

    // No classes today: say why, and point at the next class day.
    if (current == null && next == null) {
      final upcoming = upcomingDay;
      final off = dayOff;
      final unlisted = off == null && !calendarListed;
      final exam = papers.firstOrNull;
      final offMark = exam != null
          ? CalendarMark.exam
          : off == null
          ? (unlisted ? CalendarMark.holiday : null)
          : markOf(off);
      final (icon, tone) = switch (offMark) {
        CalendarMark.exam => (FLucideIcons.penLine, colors.app.warning),
        CalendarMark.holiday => (FLucideIcons.partyPopper, colors.app.success),
        CalendarMark() => (FLucideIcons.calendarOff, colors.app.accentTone),
        null => (FLucideIcons.sunMedium, colors.app.accentTone),
      };
      final typography = context.theme.typography;
      final title = exam != null
          ? '${exam.exam.courseName.trim()}'
                '${papers.length > 1 ? ' +${papers.length - 1} more' : ''}'
          : off != null
          ? titleOf(off)
          : unlisted
          ? 'Holiday'
          // The calendar holds classes, you just have none today.
          : 'Free day';
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
                  if (countdown case final countdown?) ...[
                    const SizedBox(height: Space.sm),
                    Text(
                      countdown,
                      maxLines: 1,
                      overflow: .ellipsis,
                      style: typography.body.xs.nums(
                        color: colors.mutedForeground,
                      ),
                    ),
                  ],
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
              labFat: labFats.contains(current),
              lastClass: isLastClass(current),
              progress:
                  (_timeOfDay(now) - minutesOf(current.startTime)) /
                  (minutesOf(current.endTime) - minutesOf(current.startTime)),
              record: labFats.contains(current)
                  ? null
                  : attendanceForSlot(attendance, current),
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
              labFat: labFats.contains(next),
              lastClass: isLastClass(next),
              record: labFats.contains(next)
                  ? null
                  : attendanceForSlot(attendance, next),
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
    this.labFat = false,
    this.lastClass = false,
  });

  final TimetableSlot slot;
  final Widget badge;
  final bool labFat;
  final bool lastClass;
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
                        _KindBadge(slot: slot, labFat: labFat),
                        const SizedBox(width: Space.sm),
                        Flexible(
                          child: Text(
                            slot.courseCode,
                            maxLines: 1,
                            overflow: .ellipsis,
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                            ),
                          ),
                        ),
                        if (lastClass) ...[
                          const SizedBox(width: Space.sm),
                          const _LastClassBadge(),
                        ],
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

/// The class kind chip, or an amber LAB FAT for a lab exam.
class _KindBadge extends StatelessWidget {
  const _KindBadge({required this.slot, required this.labFat});

  final TimetableSlot slot;
  final bool labFat;

  @override
  Widget build(BuildContext context) => labFat
      ? ToneBadge(
          label: 'LAB FAT',
          icon: FLucideIcons.flaskConical,
          tone: context.theme.colors.app.warning,
        )
      : CourseKindBadge(isLab: slot.kind == ClassKind.lab);
}

/// Marks a course's last class of the semester.
class _LastClassBadge extends StatelessWidget {
  const _LastClassBadge();

  @override
  Widget build(BuildContext context) =>
      ToneBadge.neutral(context, 'LAST CLASS');
}

/// The card once the day's classes are over: a ring with one arc per class,
/// clockwise from the top in class order, coloured by its posted attendance
/// (dim until VTOP posts it, amber for a Lab FAT), beside the counts.
class _DaySummary extends StatelessWidget {
  const _DaySummary({
    required this.classes,
    required this.labFats,
    required this.isToday,
  });

  final List<({TimetableSlot slot, ClassStatus? status})> classes;
  final Set<TimetableSlot> labFats;
  final bool isToday;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final palette = colors.app;
    final pending = colors.mutedForeground.withValues(alpha: 0.3);
    Color colorOf(({TimetableSlot slot, ClassStatus? status}) c) =>
        labFats.contains(c.slot)
        ? palette.warning.base
        : switch (c.status) {
            null => pending,
            ClassStatus.present => palette.success.base,
            ClassStatus.onDuty => palette.accent,
            ClassStatus.absent => palette.danger.base,
            ClassStatus.other => colors.mutedForeground,
          };
    final graded = [
      for (final c in classes)
        if (!labFats.contains(c.slot)) c.status,
    ];
    int count(ClassStatus? s) => graded.where((x) => x == s).length;
    final legend = [
      (count(ClassStatus.present), 'present', palette.success.base),
      (count(ClassStatus.onDuty), 'on duty', palette.accent),
      (count(ClassStatus.absent), 'absent', palette.danger.base),
      (
        labFats.length,
        labFats.length == 1 ? 'Lab FAT' : 'Lab FATs',
        palette.warning.base,
      ),
      (count(null), 'not posted yet', pending),
    ].where((e) => e.$1 > 0);

    return Surface(
      child: Row(
        children: [
          Semantics(
            label: [
              for (final c in classes)
                '${c.slot.name}, ${labFats.contains(c.slot) ? 'Lab FAT' : c.status?.name ?? 'not posted'}',
            ].join('; '),
            excludeSemantics: true,
            child: SizedBox.square(
              dimension: 68,
              child: CustomPaint(
                painter: _ArcsPainter([
                  for (final c in classes) colorOf(c),
                ], stroke: 6),
                child: Center(
                  child: Column(
                    mainAxisSize: .min,
                    children: [
                      Text(
                        '${classes.length}',
                        style: numStyle(
                          fontSize: 20,
                          height: 1,
                          fontWeight: .w800,
                          color: colors.foreground,
                        ),
                      ),
                      Text(
                        classes.length == 1 ? 'class' : 'classes',
                        style: typography.body.xs.copyWith(
                          fontSize: 9.5,
                          color: colors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(width: Space.lg),
          Expanded(
            child: Column(
              crossAxisAlignment: .start,
              children: [
                Text(
                  isToday ? 'Done for today' : 'Classes done',
                  style: typography.body.md.copyWith(
                    height: 1.25,
                    fontWeight: .w700,
                    color: colors.foreground,
                  ),
                ),
                const SizedBox(height: Space.sm),
                Wrap(
                  spacing: Space.md,
                  runSpacing: Space.xs,
                  children: [
                    for (final (n, label, color) in legend)
                      Row(
                        mainAxisSize: .min,
                        children: [
                          Dot(size: 8, color: color),
                          const SizedBox(width: Space.xs + 2),
                          Text.rich(
                            TextSpan(
                              children: [
                                TextSpan(
                                  text: '$n',
                                  style: TextStyle(
                                    fontWeight: .w700,
                                    color: colors.foreground,
                                  ),
                                ),
                                TextSpan(text: ' $label'),
                              ],
                            ),
                            style: typography.body.xs.nums(
                              color: colors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// A ring cut into one arc per colour, clockwise from the top.
class _ArcsPainter extends CustomPainter {
  _ArcsPainter(this.arcs, {required this.stroke});

  final List<Color> arcs;
  final double stroke;

  @override
  void paint(Canvas canvas, Size size) {
    if (arcs.isEmpty) return;
    final radius = (size.shortestSide - stroke) / 2;
    final rect = Rect.fromCircle(
      center: size.center(Offset.zero),
      radius: radius,
    );
    // Round caps reach past each arc's end; the gap leaves room for both.
    final gap = arcs.length == 1 ? 0.0 : stroke / radius + 0.14;
    final sweep = (2 * math.pi - gap * arcs.length) / arcs.length;
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = arcs.length == 1 ? StrokeCap.butt : StrokeCap.round;
    for (final (i, color) in arcs.indexed) {
      canvas.drawArc(
        rect,
        -math.pi / 2 + gap / 2 + i * (sweep + gap),
        sweep,
        false,
        paint..color = color,
      );
    }
  }

  @override
  bool shouldRepaint(_ArcsPainter old) =>
      old.stroke != stroke || !listEquals(old.arcs, arcs);
}
