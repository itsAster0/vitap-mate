// One row of the day's list: a class or a break, its rail and attendance.
part of '../agenda_timetable_view.dart';

class _ClassRow extends HookWidget {
  const _ClassRow({
    required this.slot,
    required this.status,
    required this.record,
    required this.minuteNow,
    required this.isFirst,
    required this.isLast,
  });

  final TimetableSlot slot;
  final AgendaClassStatus status;
  final AttendanceRecord? record;

  /// Minutes since midnight when this is today's schedule, else null.
  final int? minuteNow;
  final bool isFirst;
  final bool isLast;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final done = status == AgendaClassStatus.completed;
    final live = status == AgendaClassStatus.current;
    final expanded = useState(false);
    final minutes = minutesOf(slot.endTime) - minutesOf(slot.startTime);
    // Right side of the room line: when it starts, how long is left, or done.
    // Other days just show the length.
    final nowMin = minuteNow;
    final timeHint = nowMin == null
        ? _compactMinutes(minutes)
        : done
        ? 'done'
        : live
        ? '${_compactMinutes(minutesOf(slot.endTime) - nowMin)} left'
        : 'in ${_compactMinutes(minutesOf(slot.startTime) - nowMin)}';
    final creditValue = double.tryParse(slot.credits.trim());
    final credits = creditValue == null || creditValue <= 0
        ? ''
        : creditValue == creditValue.roundToDouble()
        ? creditValue.toStringAsFixed(0)
        : creditValue.toString();

    return AnimatedOpacity(
      duration: Motion.slow,
      opacity: done ? 0.55 : 1,
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: .stretch,
          children: [
            SizedBox(
              width: 62,
              child: Padding(
                padding: const .only(top: 14),
                child: Column(
                  crossAxisAlignment: .end,
                  children: [
                    FittedBox(
                      fit: .scaleDown,
                      alignment: .topRight,
                      child: Column(
                        crossAxisAlignment: .end,
                        children: [
                          Text(
                            to12H(slot.startTime, context),
                            maxLines: 1,
                            softWrap: false,
                            style: typography.body.xs.nums(
                              fontWeight: .w600,
                              color: live
                                  ? colors.app.accent
                                  : colors.foreground,
                            ),
                          ),
                          Text(
                            to12H(slot.endTime, context),
                            maxLines: 1,
                            softWrap: false,
                            style: typography.body.xs.nums(
                              color: colors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
                    ),
                    // Time status sits with the times: countdown, left, or done.
                    const SizedBox(height: Space.sm),
                    FittedBox(
                      fit: .scaleDown,
                      alignment: .topRight,
                      child: Text(
                        timeHint,
                        maxLines: 1,
                        softWrap: false,
                        style: typography.body.xs.nums(
                          fontSize: 11,
                          fontWeight: done ? .w500 : .w600,
                          color: live || status == AgendaClassStatus.next
                              ? colors.app.accentTone.onSubtle
                              : colors.mutedForeground,
                        ),
                      ),
                    ),
                    if (record != null) ...[
                      const SizedBox(height: Space.md),
                      _Attendance(record: record!, stacked: true),
                    ],
                  ],
                ),
              ),
            ),
            _Rail(status: status, isFirst: isFirst, isLast: isLast),
            Expanded(
              child: Padding(
                padding: const .symmetric(vertical: Space.xs),
                child: Surface(
                  onPress: () => expanded.value = !expanded.value,
                  semanticsLabel:
                      '${slot.name}, ${expanded.value ? 'collapse' : 'show details'}',
                  padding: const .fromLTRB(
                    Space.md + 2,
                    Space.md,
                    Space.md,
                    Space.md,
                  ),
                  borderColor: live
                      ? colors.app.accent.withValues(alpha: 0.45)
                      : null,
                  color: live ? colors.app.accentTone.subtle : null,
                  child: Column(
                    crossAxisAlignment: .start,
                    children: [
                      // Line 1: name + status
                      Row(
                        crossAxisAlignment: .start,
                        children: [
                          Expanded(
                            child: Text(
                              slot.name,
                              maxLines: 2,
                              overflow: .ellipsis,
                              style: typography.body.md.copyWith(
                                height: 1.25,
                                fontWeight: .w600,
                                color: colors.foreground,
                              ),
                            ),
                          ),
                          if (live || status == AgendaClassStatus.next) ...[
                            const SizedBox(width: Space.sm),
                            ToneBadge(
                              label: live ? 'NOW' : 'NEXT',
                              tone: colors.app.accentTone,
                              solid: live,
                            ),
                          ],
                        ],
                      ),
                      const SizedBox(height: Space.sm),
                      // Line 2: block and room
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              '${slot.block} · Room ${slot.roomNo}',
                              maxLines: 1,
                              overflow: .ellipsis,
                              style: typography.body.xs.copyWith(
                                color: colors.mutedForeground,
                              ),
                            ),
                          ),
                          AnimatedRotation(
                            turns: expanded.value ? 0.5 : 0,
                            duration: Motion.medium,
                            child: Icon(
                              FLucideIcons.chevronDown,
                              size: 16,
                              color: colors.mutedForeground,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: Space.sm),
                      // Line 3: class kind and course code
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
                      AnimatedSize(
                        duration: Motion.medium,
                        curve: Curves.easeOutCubic,
                        alignment: .topCenter,
                        child: expanded.value
                            ? Padding(
                                padding: const .only(top: Space.md),
                                child: Column(
                                  crossAxisAlignment: .start,
                                  children: [
                                    Container(height: 1, color: colors.border),
                                    const SizedBox(height: Space.md),
                                    if (slot.faculty.trim().isNotEmpty) ...[
                                      _MetaLine(
                                        icon: FLucideIcons.user,
                                        text: facultyName(slot.faculty),
                                      ),
                                      const SizedBox(height: Space.xs + 2),
                                    ],
                                    _MetaLine(
                                      icon: FLucideIcons.hash,
                                      text: 'Slot ${slot.slot}',
                                    ),
                                    const SizedBox(height: Space.xs + 2),
                                    _MetaLine(
                                      icon: FLucideIcons.clock,
                                      text: formatMinutes(minutes),
                                    ),
                                    if (credits.isNotEmpty) ...[
                                      const SizedBox(height: Space.xs + 2),
                                      _MetaLine(
                                        icon: FLucideIcons.award,
                                        text:
                                            '$credits ${credits == '1' ? 'credit' : 'credits'}',
                                      ),
                                    ],
                                  ],
                                ),
                              )
                            : const SizedBox(width: double.infinity),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Timeline rail: a line through a dot marking each class's status.
class _Rail extends StatelessWidget {
  const _Rail({
    required this.status,
    required this.isFirst,
    required this.isLast,
  });

  final AgendaClassStatus status;
  final bool isFirst;
  final bool isLast;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final accent = colors.app.accent;
    final dot = switch (status) {
      .current => accent,
      .completed => colors.mutedForeground,
      _ => colors.background,
    };
    final ring = switch (status) {
      .current || .next => accent,
      .completed => colors.mutedForeground,
      .upcoming => colors.border,
    };
    return SizedBox(
      width: 28,
      child: Stack(
        alignment: .topCenter,
        children: [
          Positioned(
            top: isFirst ? 18 : 0,
            bottom: isLast ? null : 0,
            height: isLast ? 18 : null,
            child: Container(width: 1.5, color: colors.border),
          ),
          Positioned(
            top: 13,
            child: AnimatedContainer(
              duration: Motion.medium,
              width: 11,
              height: 11,
              decoration: BoxDecoration(
                color: dot,
                shape: .circle,
                border: .all(color: ring, width: 2),
                boxShadow: status == AgendaClassStatus.current
                    ? [
                        BoxShadow(
                          color: accent.withValues(alpha: 0.35),
                          blurRadius: 8,
                          spreadRadius: 1,
                        ),
                      ]
                    : null,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// A small ring filled to the attendance percentage, coloured by the 75%
/// status. Inline (a class card's last line): "84% · skip 3" beside it.
/// [stacked] (the time column): the percentage, with the hint under it.
class _Attendance extends ConsumerWidget {
  const _Attendance({required this.record, this.stacked = false});

  final AttendanceRecord record;
  final bool stacked;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hideSkips = !ref.watch(showSkipAdviceProvider);
    final standing = AttendanceStanding.sessions(record);
    final tone = attendanceTone(
      context,
      safe: standing.isSafe,
      atEdge: standing.canSkip == 0,
    );
    final hint = !standing.isSafe
        ? 'need ${standing.mustAttend}'
        : standing.canSkip == 0
        ? "don't skip"
        : 'skip ${standing.canSkip}';
    final percent = '${standing.displayPercent.round()}%';
    TextStyle style(double size, FontWeight weight, Color color) =>
        numStyle(fontSize: size, fontWeight: weight, color: color);
    final ring = Row(
      mainAxisSize: .min,
      children: [
        ProgressRing(
          value: standing.displayPercent / 100,
          color: tone.base,
          size: stacked ? 13 : 15,
          stroke: 2.5,
        ),
        SizedBox(width: stacked ? 5 : 6),
        Text(
          stacked || hideSkips ? percent : '$percent · $hint',
          style: stacked
              ? style(12, .w700, tone.onSubtle)
              : style(11.5, .w600, tone.onSubtle),
        ),
      ],
    );
    return Semantics(
      label: '$percent attendance${hideSkips ? '' : ', ${standing.advice}'}',
      excludeSemantics: true,
      child: !stacked
          ? ring
          : FittedBox(
              fit: .scaleDown,
              alignment: .topRight,
              child: Column(
                crossAxisAlignment: .end,
                children: [
                  ring,
                  if (!hideSkips) ...[
                    const SizedBox(height: 1),
                    Text(
                      hint,
                      style: style(
                        10.5,
                        .w500,
                        tone.onSubtle.withValues(alpha: 0.8),
                      ),
                    ),
                  ],
                ],
              ),
            ),
    );
  }
}

class _MetaLine extends StatelessWidget {
  const _MetaLine({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Row(
      children: [
        Icon(icon, size: 14, color: colors.mutedForeground),
        const SizedBox(width: Space.xs + 2),
        Expanded(
          child: Text(
            text,
            maxLines: 1,
            overflow: .ellipsis,
            style: context.theme.typography.body.sm.copyWith(
              color: colors.mutedForeground,
            ),
          ),
        ),
      ],
    );
  }
}

class _IconTile extends StatelessWidget {
  const _IconTile({required this.icon, required this.tone});

  final IconData icon;
  final Tone tone;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 40,
      height: 40,
      decoration: BoxDecoration(
        color: tone.subtle,
        borderRadius: .circular(Radii.md),
      ),
      child: Icon(icon, size: 20, color: tone.base),
    );
  }
}

/// VIT leaves 10 minutes between every class; only longer gaps are breaks.
const _minBreak = 20;

class _BreakRow extends StatelessWidget {
  const _BreakRow({
    required this.previousEnd,
    required this.nextStart,
    required this.isNow,
    required this.minuteNow,
    required this.isLongest,
  });

  final String previousEnd;
  final String nextStart;
  final bool isNow;
  final int minuteNow;
  final bool isLongest;

  @override
  Widget build(BuildContext context) {
    final gap = minutesOf(nextStart) - minutesOf(previousEnd);
    if (gap < _minBreak) return const SizedBox.shrink();
    final colors = context.theme.colors;
    final color = isNow
        ? colors.app.accentTone.onSubtle
        : colors.mutedForeground;
    final until = to12H(nextStart, context);
    final length = isNow
        ? '${_compactMinutes(minutesOf(nextStart) - minuteNow)} left'
        : _compactMinutes(gap);

    return SizedBox(
      height: 34,
      child: Row(
        children: [
          // The tag sits in the time column so the break text gets the row.
          SizedBox(
            width: 62,
            child: isLongest && !isNow
                ? Align(
                    alignment: .centerRight,
                    child: Padding(
                      padding: const .only(left: Space.xs),
                      child: FittedBox(
                        fit: .scaleDown,
                        child: ToneBadge.neutral(context, 'LONGEST'),
                      ),
                    ),
                  )
                : null,
          ),
          SizedBox(
            width: 28,
            child: Center(
              child: CustomPaint(
                size: const Size(1.5, 34),
                painter: _DashPainter(colors.border),
              ),
            ),
          ),
          Icon(FLucideIcons.coffee, size: 14, color: color),
          const SizedBox(width: Space.sm),
          Flexible(
            child: Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: 'Free until $until',
                    style: TextStyle(
                      fontWeight: isNow ? .w600 : .w500,
                      color: isNow ? color : colors.foreground,
                    ),
                  ),
                  TextSpan(text: ' · $length'),
                ],
              ),
              maxLines: 1,
              overflow: .ellipsis,
              style: context.theme.typography.body.xs.nums(color: color),
            ),
          ),
        ],
      ),
    );
  }
}

/// 129 → "2h 9m", 45 → "45m"; short enough to share a row with a tag.
String _compactMinutes(int minutes) {
  final h = minutes ~/ 60;
  final m = minutes % 60;
  if (h == 0) return '${m}m';
  return m == 0 ? '${h}h' : '${h}h ${m}m';
}

class _DashPainter extends CustomPainter {
  _DashPainter(this.color);

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = size.width;
    for (var y = 0.0; y < size.height; y += 6) {
      canvas.drawLine(Offset(0, y), Offset(0, y + 3), paint);
    }
  }

  @override
  bool shouldRepaint(_DashPainter old) => old.color != color;
}
