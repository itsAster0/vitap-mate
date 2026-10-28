// Above the day: the date heading, the day bar and the week strip.
part of '../agenda_timetable_view.dart';

class _DateHeading extends StatelessWidget {
  const _DateHeading({
    required this.date,
    required this.isToday,
    required this.slots,
    required this.onToday,
    this.note,
  });

  final DateTime date;
  final bool isToday;
  final List<TimetableSlot> slots;
  final VoidCallback onToday;

  /// The calendar's word on the day: "CAT - II", "Lab FAT", "Deepavali".
  final String? note;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final summary = [
      DateFormat('d MMMM').format(date),
      ?note,
      if (slots.isNotEmpty)
        '${to12H(slots.first.startTime, context)} – ${to12H(slots.last.endTime, context)}',
    ].join('  ·  ');

    // The Today button sits beside the day name, so the summary below gets the
    // full width instead of ellipsing beside it.
    return Column(
      crossAxisAlignment: .start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                DateFormat('EEEE').format(date),
                maxLines: 1,
                overflow: .ellipsis,
                style: typography.display.xl2.copyWith(
                  height: 1.05,
                  fontWeight: .w800,
                  letterSpacing: -0.5,
                  color: colors.foreground,
                ),
              ),
            ),
            AnimatedSwitcher(
              duration: Motion.medium,
              child: isToday
                  ? const SizedBox.shrink()
                  : FButton(
                      variant: .outline,
                      size: .sm,
                      mainAxisSize: .min,
                      prefix: const Icon(FLucideIcons.undo2),
                      onPress: onToday,
                      child: const Text('Today'),
                    ),
            ),
          ],
        ),
        const SizedBox(height: Space.xs),
        Text(
          summary,
          maxLines: 1,
          overflow: .ellipsis,
          style: typography.body.sm.nums(color: colors.mutedForeground),
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
            clipBehavior: .none,
            alignment: .centerLeft,
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
                        borderRadius: .circular(3),
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
                      borderRadius: .circular(1),
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
    this.examDays = const {},
  });

  final List<DateTime> dates;
  final int today;
  final ValueNotifier<int> selectedDay;

  /// Classes per weekday, Monday first.
  final List<int> classCounts;

  /// ISO weekdays with one of your exam papers, marked in amber.
  final Set<int> examDays;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    Widget dot(Color color) => Padding(
      padding: const .symmetric(horizontal: 1),
      child: Dot(size: 3, color: color),
    );

    Widget tile(int index, DateTime date) {
      final day = index + 1;
      final selected = selectedDay.value == day;
      final isToday = today == day;
      final count = classCounts[index];
      // Class-free days recede unless selected or today.
      final exam = examDays.contains(day);
      final free = count == 0 && !exam && !selected && !isToday;
      return PressScale(
        scale: 0.94,
        semanticsLabel: DateFormat('EEEE, d MMMM').format(date),
        onPress: () => selectedDay.value = day,
        child: AnimatedContainer(
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
          height: 54,
          margin: const .symmetric(horizontal: 2),
          decoration: BoxDecoration(
            color: selected ? colors.primary : const Color(0x00000000),
            borderRadius: .circular(Radii.md),
            border: .all(
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
              mainAxisAlignment: .center,
              children: [
                Text(
                  DateFormat('EEE').format(date).toUpperCase(),
                  style: TextStyle(
                    fontSize: 10,
                    letterSpacing: 0.6,
                    fontWeight: .w600,
                    color: selected
                        ? colors.primaryForeground.withValues(alpha: 0.7)
                        : colors.mutedForeground,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  '${date.day}',
                  style: numStyle(
                    fontSize: 17,
                    fontWeight: .w700,
                    color: selected
                        ? colors.primaryForeground
                        : isToday
                        ? colors.app.accent
                        : colors.foreground,
                  ),
                ),
                const SizedBox(height: 5),
                // One dot per class (capped), so the week's load reads at a
                // glance.
                SizedBox(
                  height: 3,
                  child: Row(
                    mainAxisSize: .min,
                    children: [
                      for (var i = 0; i < count.clamp(0, 6); i++)
                        dot(
                          selected
                              ? colors.primaryForeground
                              : colors.mutedForeground,
                        ),
                      if (exam) dot(colors.app.warning.base),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    }

    return Row(
      children: [
        for (final (index, date) in dates.indexed)
          Expanded(child: tile(index, date)),
      ],
    );
  }
}
