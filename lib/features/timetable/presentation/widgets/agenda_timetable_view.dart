import 'dart:math' as math;
import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/timetable/presentation/utils/time_format.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum AgendaClassStatus { completed, current, next, upcoming }

/// Day view of the timetable: a live "now / next" panel on top of the day's
/// schedule, with attendance guidance on every class.
class AgendaTimetableView extends HookWidget {
  const AgendaTimetableView({
    super.key,
    required this.selectedDay,
    required this.classDays,
    required this.slotsForDay,
    this.attendance = const [],
  });

  /// ISO weekday (1 = Monday) being shown.
  final ValueNotifier<int> selectedDay;
  final Set<int> classDays;
  final List<TimetableSlot> Function(int weekday) slotsForDay;
  final List<AttendanceRecord> attendance;

  @override
  Widget build(BuildContext context) {
    final now = useState(DateTime.now());
    useEffect(() {
      // Status changes on minute boundaries; sync the first tick to one.
      Timer? periodic;
      final first = Timer(Duration(seconds: 60 - DateTime.now().second), () {
        now.value = DateTime.now();
        periodic = Timer.periodic(
          const Duration(minutes: 1),
          (_) => now.value = DateTime.now(),
        );
      });
      return () {
        first.cancel();
        periodic?.cancel();
      };
    }, const []);

    final slots = slotsForDay(selectedDay.value);
    final weekDates = _weekDates(now.value);
    final selectedDate = weekDates[selectedDay.value - 1];
    final isToday = selectedDay.value == now.value.weekday;
    final minuteNow = _timeOfDay(now.value);

    final current = isToday
        ? _firstWhereOrNull(
            slots,
            (s) =>
                minutesOf(s.startTime) <= minuteNow &&
                minutesOf(s.endTime) > minuteNow,
          )
        : null;
    final next = _firstWhereOrNull(
      slots,
      (s) => !isToday || minutesOf(s.startTime) > minuteNow,
    );
    final remaining = isToday
        ? slots.where((s) => minutesOf(s.endTime) > minuteNow).length
        : slots.length;

    // Real breaks (VIT leaves 10 minutes between classes); the longest one is
    // tagged when the day has more than one.
    final gaps = [
      for (var i = 1; i < slots.length; i++)
        minutesOf(slots[i].startTime) - minutesOf(slots[i - 1].endTime),
    ];
    final realGaps = gaps.where((g) => g >= _minBreak).toList();
    final longestGap = realGaps.length > 1
        ? realGaps.reduce((a, b) => a > b ? a : b)
        : null;

    // Previous day index tells the switcher which way to slide.
    final previousDay = usePrevious(selectedDay.value) ?? selectedDay.value;
    final forward = selectedDay.value >= previousDay;

    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.sm, Space.sm, Space.sm, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _DateHeading(
            date: selectedDate,
            isToday: isToday,
            slots: slots,
            onToday: () => selectedDay.value = now.value.weekday,
          ),
          if (slots.isNotEmpty) ...[
            const SizedBox(height: Space.md),
            _DayBar(slots: slots, minuteNow: isToday ? minuteNow : null),
          ],
          const SizedBox(height: Space.md),
          _WeekStrip(
            dates: weekDates,
            today: now.value.weekday,
            selectedDay: selectedDay,
            classCounts: [for (var d = 1; d <= 7; d++) slotsForDay(d).length],
          ),
          const SizedBox(height: Space.md),
          AnimatedSwitcher(
            duration: Motion.slow,
            // Old day fades out in the first third, new day fades in after,
            // so the two never overlap.
            switchInCurve: const Interval(0.35, 1, curve: Curves.easeOutCubic),
            switchOutCurve: const Interval(0.65, 1, curve: Curves.easeIn),
            transitionBuilder: (child, animation) {
              final incoming = child.key == ValueKey(selectedDay.value);
              final dx = (incoming == forward ? 1 : -1) * 0.04;
              return FadeTransition(
                opacity: animation,
                child: SlideTransition(
                  position: Tween(
                    begin: Offset(dx, 0),
                    end: Offset.zero,
                  ).animate(animation),
                  child: child,
                ),
              );
            },
            layoutBuilder: (currentChild, previous) => Stack(
              alignment: Alignment.topCenter,
              children: [...previous, ?currentChild],
            ),
            child: Column(
              key: ValueKey(selectedDay.value),
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _FocusPanel(
                  current: current,
                  next: next,
                  now: now.value,
                  isToday: isToday,
                  slots: slots,
                  attendance: attendance,
                  upcomingDay: slots.isEmpty || (isToday && next == null)
                      ? _nextClassDay(selectedDay.value)
                      : null,
                ),
                if (slots.isNotEmpty)
                  SectionHeader(
                    title: isToday ? 'Today' : 'Schedule',
                    trailing: Text(
                      slots.isEmpty
                          ? ''
                          : isToday && remaining < slots.length
                          ? '$remaining of ${slots.length} left'
                          : '${slots.length} ${slots.length == 1 ? 'class' : 'classes'}',
                    ),
                  ),
                for (var i = 0; i < slots.length; i++) ...[
                  if (i > 0)
                    _BreakRow(
                      previousEnd: slots[i - 1].endTime,
                      nextStart: slots[i].startTime,
                      isNow:
                          isToday &&
                          minutesOf(slots[i - 1].endTime) <= minuteNow &&
                          minutesOf(slots[i].startTime) > minuteNow,
                      minuteNow: minuteNow,
                      isLongest: gaps[i - 1] == longestGap,
                    ),
                  EnterFade(
                    index: i,
                    child: _ClassRow(
                      slot: slots[i],
                      status: _statusFor(
                        slots[i],
                        current,
                        next,
                        minuteNow,
                        isToday,
                      ),
                      record: attendanceForSlot(attendance, slots[i]),
                      minuteNow: isToday ? minuteNow : null,
                      isFirst: i == 0,
                      isLast: i == slots.length - 1,
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

  /// The next weekday after [from] that has classes, with its first class.
  ({int weekday, TimetableSlot first})? _nextClassDay(int from) {
    for (var offset = 1; offset <= 7; offset++) {
      final day = (from - 1 + offset) % 7 + 1;
      final slots = slotsForDay(day);
      if (slots.isNotEmpty) return (weekday: day, first: slots.first);
    }
    return null;
  }
}

class _DateHeading extends StatelessWidget {
  const _DateHeading({
    required this.date,
    required this.isToday,
    required this.slots,
    required this.onToday,
  });

  final DateTime date;
  final bool isToday;
  final List<TimetableSlot> slots;
  final VoidCallback onToday;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final summary = [
      DateFormat('d MMMM').format(date),
      if (slots.isNotEmpty)
        '${to12H(slots.first.startTime, context)} – ${to12H(slots.last.endTime, context)}',
    ].join('  ·  ');

    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                DateFormat('EEEE').format(date),
                style: typography.display.xl2.copyWith(
                  height: 1.05,
                  fontWeight: FontWeight.w800,
                  letterSpacing: -0.5,
                  color: colors.foreground,
                ),
              ),
              const SizedBox(height: Space.xs),
              Text(
                summary,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: typography.body.sm.copyWith(
                  color: colors.mutedForeground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ],
          ),
        ),
        AnimatedSwitcher(
          duration: Motion.medium,
          child: isToday
              ? const SizedBox.shrink()
              : FButton(
                  variant: FButtonVariant.outline,
                  size: FButtonSizeVariant.sm,
                  mainAxisSize: MainAxisSize.min,
                  prefix: const Icon(FLucideIcons.undo2),
                  onPress: onToday,
                  child: const Text('Today'),
                ),
        ),
      ],
    );
  }
}

/// The day's shape on one thin line: a segment per class along the span from
/// first class to last, filled up to now, with a marker at the current time.
class _DayBar extends StatelessWidget {
  const _DayBar({required this.slots, required this.minuteNow});

  final List<TimetableSlot> slots;

  /// Minutes since midnight when this is today, else null (nothing elapsed).
  final int? minuteNow;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final start = minutesOf(slots.first.startTime);
    final end = minutesOf(slots.last.endTime);
    final span = (end - start).clamp(1, 24 * 60);
    final now = minuteNow;

    return SizedBox(
      height: 10,
      child: LayoutBuilder(
        builder: (context, c) {
          double x(int minute) =>
              ((minute - start) / span).clamp(0.0, 1.0) * c.maxWidth;
          return Stack(
            clipBehavior: Clip.none,
            alignment: Alignment.centerLeft,
            children: [
              // Track for the free time between classes.
              Positioned(
                left: 0,
                right: 0,
                top: 4,
                child: Container(height: 2, color: colors.secondary),
              ),
              for (final slot in slots)
                Builder(
                  builder: (context) {
                    final s = minutesOf(slot.startTime);
                    final e = minutesOf(slot.endTime);
                    final tone = slot.kind == ClassKind.lab
                        ? colors.app.lab
                        : colors.app.accentTone;
                    final elapsed = now == null
                        ? 0.0
                        : ((now - s) / (e - s)).clamp(0.0, 1.0);
                    final width = math.max(3.0, x(e) - x(s) - 2);
                    return Positioned(
                      left: x(s) + 1,
                      top: 2,
                      width: width,
                      height: 6,
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(3),
                        child: Stack(
                          children: [
                            Positioned.fill(
                              child: ColoredBox(
                                color: tone.base.withValues(alpha: 0.3),
                              ),
                            ),
                            Positioned(
                              left: 0,
                              top: 0,
                              bottom: 0,
                              width: width * elapsed,
                              child: ColoredBox(color: tone.base),
                            ),
                          ],
                        ),
                      ),
                    );
                  },
                ),
              if (now != null && now >= start && now <= end)
                Positioned(
                  left: x(now) - 1,
                  top: -2,
                  width: 2,
                  height: 14,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: colors.foreground,
                      borderRadius: BorderRadius.circular(1),
                    ),
                  ),
                ),
            ],
          );
        },
      ),
    );
  }
}

class _WeekStrip extends StatelessWidget {
  const _WeekStrip({
    required this.dates,
    required this.today,
    required this.selectedDay,
    required this.classCounts,
  });

  final List<DateTime> dates;
  final int today;
  final ValueNotifier<int> selectedDay;

  /// Classes per weekday, Monday first.
  final List<int> classCounts;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Row(
      children: [
        for (var index = 0; index < dates.length; index++)
          Expanded(
            child: Builder(
              builder: (context) {
                final day = index + 1;
                final selected = selectedDay.value == day;
                final isToday = today == day;
                final count = classCounts[index];
                // Class-free days recede unless selected or today.
                final free = count == 0 && !selected && !isToday;
                final fg = selected
                    ? colors.primaryForeground
                    : isToday
                    ? colors.app.accent
                    : colors.foreground;
                return PressScale(
                  scale: 0.94,
                  semanticsLabel: DateFormat(
                    'EEEE, d MMMM',
                  ).format(dates[index]),
                  onPress: () => selectedDay.value = day,
                  child: AnimatedContainer(
                    duration: Motion.medium,
                    curve: Curves.easeOutCubic,
                    height: 54,
                    margin: const EdgeInsets.symmetric(horizontal: 2),
                    decoration: BoxDecoration(
                      color: selected
                          ? colors.primary
                          : const Color(0x00000000),
                      borderRadius: BorderRadius.circular(Radii.md),
                      border: Border.all(
                        color: selected
                            ? colors.primary
                            : isToday
                            ? colors.app.accent.withValues(alpha: 0.5)
                            : const Color(0x00000000),
                      ),
                    ),
                    child: AnimatedOpacity(
                      duration: Motion.medium,
                      opacity: free ? 0.35 : 1,
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Text(
                            DateFormat(
                              'EEE',
                            ).format(dates[index]).toUpperCase(),
                            style: TextStyle(
                              fontSize: 10,
                              letterSpacing: 0.6,
                              fontWeight: FontWeight.w600,
                              color: selected
                                  ? colors.primaryForeground.withValues(
                                      alpha: 0.7,
                                    )
                                  : colors.mutedForeground,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            '${dates[index].day}',
                            style: TextStyle(
                              fontSize: 17,
                              fontWeight: FontWeight.w700,
                              color: fg,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                          const SizedBox(height: 5),
                          // One dot per class (capped), so the week's load
                          // reads at a glance.
                          SizedBox(
                            height: 3,
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                for (var i = 0; i < count.clamp(0, 6); i++)
                                  Container(
                                    width: 3,
                                    height: 3,
                                    margin: const EdgeInsets.symmetric(
                                      horizontal: 1,
                                    ),
                                    decoration: BoxDecoration(
                                      shape: BoxShape.circle,
                                      color: selected
                                          ? colors.primaryForeground
                                          : colors.mutedForeground,
                                    ),
                                  ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
      ],
    );
  }
}

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
  });

  final TimetableSlot? current;
  final TimetableSlot? next;
  final DateTime now;
  final bool isToday;
  final List<TimetableSlot> slots;
  final List<AttendanceRecord> attendance;
  final ({int weekday, TimetableSlot first})? upcomingDay;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;

    // Day is over, or there's nothing today: point at the next class day.
    if (current == null && next == null) {
      final upcoming = upcomingDay;
      return Surface(
        child: Row(
          children: [
            _IconTile(
              icon: slots.isEmpty
                  ? FLucideIcons.sunMedium
                  : FLucideIcons.circleCheckBig,
              tone: slots.isEmpty ? colors.app.accentTone : colors.app.success,
            ),
            const SizedBox(width: Space.md),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    slots.isEmpty ? 'Free day' : "You're done for today",
                    style: context.theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w700,
                      color: colors.foreground,
                    ),
                  ),
                  if (upcoming != null) ...[
                    const SizedBox(height: 2),
                    Text(
                      '${_dayLabel(upcoming.weekday, now)} starts '
                      '${to12H(upcoming.first.startTime, context)} · '
                      '${upcoming.first.name}',
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: context.theme.typography.body.sm.copyWith(
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
      padding: EdgeInsets.zero,
      borderColor: current != null
          ? colors.app.accent.withValues(alpha: 0.45)
          : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (current != null)
            _FocusClass(
              slot: current!,
              badge: ToneBadge(
                label: 'NOW',
                tone: colors.app.accentTone,
                solid: true,
              ),
              countdown: _Countdown(
                target: _at(now, current!.endTime),
                prefix: 'Ends in',
              ),
              progress:
                  (_timeOfDay(now) - minutesOf(current!.startTime)) /
                  (minutesOf(current!.endTime) - minutesOf(current!.startTime)),
              record: attendanceForSlot(attendance, current!),
              large: true,
            ),
          if (current != null && next != null) ...[
            Container(height: 1, color: colors.border),
            _NextStrip(slot: next!, now: now),
          ],
          if (current == null && next != null)
            _FocusClass(
              slot: next!,
              badge: ToneBadge(
                label: isToday ? 'NEXT' : 'FIRST CLASS',
                tone: colors.app.accentTone,
              ),
              countdown: isToday
                  ? _Countdown(
                      target: _at(now, next!.startTime),
                      prefix: 'Starts in',
                    )
                  : null,
              record: attendanceForSlot(attendance, next!),
              large: true,
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
    this.large = false,
  });

  final TimetableSlot slot;
  final Widget badge;
  final Widget? countdown;
  final double? progress;
  final AttendanceRecord? record;
  final bool large;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Padding(
      padding: const EdgeInsets.all(Space.lg),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              badge,
              const SizedBox(width: Space.sm),
              if (countdown != null) Expanded(child: countdown!),
              if (countdown == null) const Spacer(),
              Text(
                '${to12H(slot.startTime, context)} – ${to12H(slot.endTime, context)}',
                style: typography.body.xs.copyWith(
                  fontWeight: FontWeight.w600,
                  color: colors.mutedForeground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.md),
          Text(
            slot.name,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: typography.body.xl.copyWith(
              height: 1.2,
              fontWeight: FontWeight.w700,
              letterSpacing: -0.2,
              color: colors.foreground,
            ),
          ),
          const SizedBox(height: Space.md),
          // Details on the left, the room as a big tile on the right — it's
          // what you look for on the way to class.
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
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
                            overflow: TextOverflow.ellipsis,
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                            ),
                          ),
                        ),
                      ],
                    ),
                    if (record != null) ...[
                      const SizedBox(height: Space.sm),
                      _InlineAttendance(record: record!),
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
        padding: const EdgeInsets.symmetric(
          horizontal: Space.sm,
          vertical: Space.sm + 2,
        ),
        decoration: BoxDecoration(
          color: colors.secondary,
          borderRadius: BorderRadius.circular(Radii.md),
          border: Border.all(color: colors.border),
        ),
        child: Column(
          children: [
            Text(
              'ROOM',
              style: TextStyle(
                fontSize: 9,
                letterSpacing: 0.8,
                fontWeight: FontWeight.w600,
                color: colors.mutedForeground,
              ),
            ),
            FittedBox(
              fit: BoxFit.scaleDown,
              child: Text(
                slot.roomNo,
                maxLines: 1,
                style: TextStyle(
                  fontSize: 26,
                  height: 1.15,
                  fontWeight: FontWeight.w800,
                  letterSpacing: -0.5,
                  color: colors.foreground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ),
            Text(
              slot.block,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 10.5,
                fontWeight: FontWeight.w600,
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
      padding: const EdgeInsets.fromLTRB(
        Space.lg,
        Space.md,
        Space.lg,
        Space.md,
      ),
      child: Row(
        children: [
          Text(
            'NEXT',
            style: typography.body.xs.copyWith(
              fontWeight: FontWeight.w700,
              letterSpacing: 0.6,
              color: colors.app.accentTone.onSubtle,
            ),
          ),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  slot.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
                Text(
                  '${to12H(slot.startTime, context)} · Room ${slot.roomNo}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
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
              fontWeight: FontWeight.w600,
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
                  fontWeight: FontWeight.w700,
                  color: context.theme.colors.foreground,
                ),
              ),
            ],
          ),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: context.theme.typography.body.xs.copyWith(
            color: context.theme.colors.mutedForeground,
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
      ),
    );
  }
}

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
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(
              width: 62,
              child: Padding(
                padding: const EdgeInsets.only(top: 14),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    FittedBox(
                      fit: BoxFit.scaleDown,
                      alignment: Alignment.topRight,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.end,
                        children: [
                          Text(
                            to12H(slot.startTime, context),
                            maxLines: 1,
                            softWrap: false,
                            style: typography.body.xs.copyWith(
                              fontWeight: FontWeight.w600,
                              color: live
                                  ? colors.app.accent
                                  : colors.foreground,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                          Text(
                            to12H(slot.endTime, context),
                            maxLines: 1,
                            softWrap: false,
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                    // Time status sits with the times: countdown, left, or done.
                    const SizedBox(height: Space.sm),
                    FittedBox(
                      fit: BoxFit.scaleDown,
                      alignment: Alignment.topRight,
                      child: Text(
                        timeHint,
                        maxLines: 1,
                        softWrap: false,
                        style: typography.body.xs.copyWith(
                          fontSize: 11,
                          fontWeight: done ? FontWeight.w500 : FontWeight.w600,
                          color: live || status == AgendaClassStatus.next
                              ? colors.app.accentTone.onSubtle
                              : colors.mutedForeground,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                    ),
                    if (record != null) ...[
                      const SizedBox(height: Space.md),
                      _GutterAttendance(record: record!),
                    ],
                  ],
                ),
              ),
            ),
            _Rail(status: status, isFirst: isFirst, isLast: isLast),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: Space.xs),
                child: Surface(
                  onPress: () => expanded.value = !expanded.value,
                  semanticsLabel:
                      '${slot.name}, ${expanded.value ? 'collapse' : 'show details'}',
                  padding: const EdgeInsets.fromLTRB(
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
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      // Line 1: name + status
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(
                            child: Text(
                              slot.name,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: typography.body.md.copyWith(
                                height: 1.25,
                                fontWeight: FontWeight.w600,
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
                              overflow: TextOverflow.ellipsis,
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
                              overflow: TextOverflow.ellipsis,
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
                        alignment: Alignment.topCenter,
                        child: expanded.value
                            ? Padding(
                                padding: const EdgeInsets.only(top: Space.md),
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
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
      AgendaClassStatus.current => accent,
      AgendaClassStatus.completed => colors.mutedForeground,
      _ => colors.background,
    };
    final ring = switch (status) {
      AgendaClassStatus.current => accent,
      AgendaClassStatus.next => accent,
      AgendaClassStatus.completed => colors.mutedForeground,
      AgendaClassStatus.upcoming => colors.border,
    };
    return SizedBox(
      width: 28,
      child: Stack(
        alignment: Alignment.topCenter,
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
                shape: BoxShape.circle,
                border: Border.all(color: ring, width: 2),
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

/// Right side of a class card's last line: a small ring filled to the
/// percentage (notched at 75%) with "84% · skip 3" beside it.
class _InlineAttendance extends StatelessWidget {
  const _InlineAttendance({required this.record});

  final AttendanceRecord record;

  @override
  Widget build(BuildContext context) {
    final standing = AttendanceStanding.of(record);
    final palette = context.theme.colors.app;
    final tone = !standing.isSafe
        ? palette.danger
        : standing.canSkip == 0
        ? palette.warning
        : palette.success;
    final hint = !standing.isSafe
        ? 'need ${standing.mustAttend}'
        : standing.canSkip == 0
        ? "don't skip"
        : 'skip ${standing.canSkip}';
    return Semantics(
      label:
          '${standing.displayPercent.round()}% attendance, ${standing.advice}',
      excludeSemantics: true,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          ProgressRing(
            value: standing.displayPercent / 100,
            color: tone.base,
            size: 15,
            stroke: 2.5,
          ),
          const SizedBox(width: 6),
          Text(
            '${standing.displayPercent.round()}% · $hint',
            style: TextStyle(
              fontSize: 11.5,
              fontWeight: FontWeight.w600,
              color: tone.onSubtle,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ],
      ),
    );
  }
}

/// Attendance for the time column: a small ring with the percentage, and the
/// skip/attend hint under it, coloured by the 75% status.
class _GutterAttendance extends StatelessWidget {
  const _GutterAttendance({required this.record});

  final AttendanceRecord record;

  @override
  Widget build(BuildContext context) {
    final standing = AttendanceStanding.of(record);
    final palette = context.theme.colors.app;
    final tone = !standing.isSafe
        ? palette.danger
        : standing.canSkip == 0
        ? palette.warning
        : palette.success;
    final hint = !standing.isSafe
        ? 'need ${standing.mustAttend}'
        : standing.canSkip == 0
        ? "don't skip"
        : 'skip ${standing.canSkip}';
    return Semantics(
      label:
          '${standing.displayPercent.round()}% attendance, ${standing.advice}',
      excludeSemantics: true,
      child: FittedBox(
        fit: BoxFit.scaleDown,
        alignment: Alignment.topRight,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                ProgressRing(
                  value: standing.displayPercent / 100,
                  color: tone.base,
                  size: 13,
                  stroke: 2.5,
                ),
                const SizedBox(width: 5),
                Text(
                  '${standing.displayPercent.round()}%',
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w700,
                    color: tone.onSubtle,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 1),
            Text(
              hint,
              style: TextStyle(
                fontSize: 10.5,
                fontWeight: FontWeight.w500,
                color: tone.onSubtle.withValues(alpha: 0.8),
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
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
            overflow: TextOverflow.ellipsis,
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
        borderRadius: BorderRadius.circular(Radii.md),
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

    return Padding(
      padding: const EdgeInsets.only(left: 62),
      child: SizedBox(
        height: 34,
        child: Row(
          children: [
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
                        fontWeight: isNow ? FontWeight.w600 : FontWeight.w500,
                        color: isNow ? color : colors.foreground,
                      ),
                    ),
                    TextSpan(text: ' · $length'),
                  ],
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: context.theme.typography.body.xs.copyWith(
                  color: color,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ),
            // The live break is already highlighted; skip the tag so the
            // countdown fits.
            if (isLongest && !isNow) ...[
              const SizedBox(width: Space.sm),
              ToneBadge.neutral(context, 'LONGEST'),
            ],
          ],
        ),
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

AgendaClassStatus _statusFor(
  TimetableSlot slot,
  TimetableSlot? current,
  TimetableSlot? next,
  int minuteNow,
  bool isToday,
) {
  if (slot == current) return AgendaClassStatus.current;
  if (slot == next && isToday) return AgendaClassStatus.next;
  if (isToday && minutesOf(slot.endTime) <= minuteNow) {
    return AgendaClassStatus.completed;
  }
  return AgendaClassStatus.upcoming;
}

T? _firstWhereOrNull<T>(Iterable<T> items, bool Function(T) test) {
  for (final item in items) {
    if (test(item)) return item;
  }
  return null;
}

String _dayLabel(int weekday, DateTime now) {
  if (weekday == now.weekday % 7 + 1) return 'Tomorrow';
  return DateFormat('EEEE').format(
    DateTime(2024, 1, weekday), // 1 Jan 2024 was a Monday.
  );
}

DateTime _at(DateTime day, String time) {
  final minutes = minutesOf(time);
  return DateTime(day.year, day.month, day.day, minutes ~/ 60, minutes % 60);
}

List<DateTime> _weekDates(DateTime now) {
  final monday = DateTime(
    now.year,
    now.month,
    now.day,
  ).subtract(Duration(days: now.weekday - DateTime.monday));
  return List.generate(7, (index) => monday.add(Duration(days: index)));
}

int _timeOfDay(DateTime value) => value.hour * 60 + value.minute;
