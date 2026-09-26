import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';

/// "What if" planner: choose how many upcoming classes to attend or skip and
/// see where the percentage lands. Current counts can be corrected too.
class AttendancePlanner extends HookWidget {
  const AttendancePlanner({
    super.key,
    required this.attended,
    required this.total,
    this.reported,
  });

  final int attended;
  final int total;

  /// VTOP's own percentage, shown as "now" until the counts are edited.
  final double? reported;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;

    final editCurrent = useState(false);
    final curAttended = useState(attended);
    final curTotal = useState(total);
    final countsEdited =
        curAttended.value != attended || curTotal.value != total;
    final current = AttendanceStanding(
      attended: curAttended.value,
      total: curTotal.value,
      reported: countsEdited ? null : reported,
    );

    final willAttend = useState(0);
    final willSkip = useState(0);
    void setPlan(int attend, int skip) {
      willAttend.value = attend;
      willSkip.value = skip;
    }

    // Start from the most useful plan: max skips, or the classes needed.
    void usefulPlan() => current.isSafe
        ? setPlan(0, current.canSkip)
        : setPlan(current.mustAttend, 0);

    useEffect(() {
      usefulPlan();
      return null;
    }, [curAttended.value, curTotal.value]);

    final predicted = AttendanceStanding(
      attended: curAttended.value + willAttend.value,
      total: curTotal.value + willAttend.value + willSkip.value,
    );
    final tone = attendanceTone(
      context,
      safe: predicted.isSafe,
      atEdge: predicted.canSkip == 0,
    );
    final nowTone = attendanceTone(
      context,
      safe: current.isSafe,
      atEdge: current.canSkip == 0,
    );

    final presets = [
      if (current.isSafe && current.canSkip > 0)
        ('Max skips', 0, current.canSkip),
      if (!current.isSafe) ('Recover', current.mustAttend, 0),
      ('Attend 5', 5, 0),
      ('Attend 10', 10, 0),
      ('Skip 1', 0, 1),
    ];

    return Padding(
      padding: const EdgeInsets.only(top: Space.lg),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Result: now → after the plan.
          Surface(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  children: [
                    _Figure(
                      label: 'NOW',
                      value: current.displayPercent,
                      decimals: current.reported != null ? 0 : 1,
                      color: nowTone.onSubtle,
                    ),
                    Expanded(
                      child: Icon(
                        FLucideIcons.arrowRight,
                        size: 20,
                        color: colors.mutedForeground,
                      ),
                    ),
                    _Figure(
                      label: 'AFTER PLAN',
                      value: predicted.percent,
                      color: tone.onSubtle,
                      alignEnd: true,
                    ),
                  ],
                ),
                const SizedBox(height: Space.md),
                SizedBox(
                  height: 14,
                  child: LayoutBuilder(
                    builder: (context, c) => Stack(
                      clipBehavior: Clip.none,
                      children: [
                        Positioned.fill(
                          child: SkipMeter(
                            percent: predicted.percent,
                            tone: tone,
                          ),
                        ),
                        // Ghost marker where you are today.
                        AnimatedPositioned(
                          duration: Motion.medium,
                          left: (c.maxWidth * current.percent / 100 - 5).clamp(
                            0.0,
                            c.maxWidth - 10,
                          ),
                          top: 2,
                          child: Container(
                            width: 10,
                            height: 10,
                            decoration: BoxDecoration(
                              shape: BoxShape.circle,
                              color: colors.background,
                              border: Border.all(
                                color: colors.mutedForeground,
                                width: 2,
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: Space.md),
                AnimatedSwitcher(
                  duration: Motion.medium,
                  child: Text(
                    predicted.isSafe
                        ? predicted.canSkip == 0
                              ? 'Right on the line — no more skips after this.'
                              : 'Still safe, with ${predicted.canSkip} more to spare.'
                        : 'Below 75% — attend ${predicted.mustAttend} more to recover.',
                    key: ValueKey('${predicted.attended}/${predicted.total}'),
                    style: typography.body.sm.copyWith(
                      fontWeight: FontWeight.w600,
                      color: predicted.isSafe && predicted.canSkip > 0
                          ? colors.foreground
                          : tone.onSubtle,
                    ),
                  ),
                ),
                Text(
                  '${predicted.attended} of ${predicted.total} classes',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: Space.md),
          // Presets
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                for (final (label, a, s) in presets)
                  Padding(
                    padding: const EdgeInsets.only(right: Space.sm),
                    child: FButton(
                      variant: willAttend.value == a && willSkip.value == s
                          ? FButtonVariant.primary
                          : FButtonVariant.outline,
                      size: FButtonSizeVariant.sm,
                      mainAxisSize: MainAxisSize.min,
                      onPress: () => setPlan(a, s),
                      child: Text(label),
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(height: Space.md),
          // Plan
          Surface(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                CountStepper(
                  label: 'Attend',
                  caption: 'Upcoming classes you go to',
                  value: willAttend.value,
                  onChanged: (v) => willAttend.value = v,
                ),
                const SizedBox(height: Space.md),
                CountStepper(
                  label: 'Skip',
                  caption: 'Upcoming classes you miss',
                  value: willSkip.value,
                  onChanged: (v) => willSkip.value = v,
                ),
                if (willAttend.value + willSkip.value > 0) ...[
                  const SizedBox(height: Space.md),
                  _PlanStrip(attend: willAttend.value, skip: willSkip.value),
                ],
              ],
            ),
          ),
          const SizedBox(height: Space.md),
          // Correct current counts
          Surface(
            padding: EdgeInsets.zero,
            child: Column(
              children: [
                PressScale(
                  scale: 0.99,
                  onPress: () {
                    if (editCurrent.value) {
                      curAttended.value = attended;
                      curTotal.value = total;
                    }
                    editCurrent.value = !editCurrent.value;
                  },
                  child: Padding(
                    padding: const EdgeInsets.all(Space.lg),
                    child: Row(
                      children: [
                        Icon(
                          FLucideIcons.pencil,
                          size: 16,
                          color: colors.mutedForeground,
                        ),
                        const SizedBox(width: Space.sm),
                        Expanded(
                          child: Text(
                            editCurrent.value
                                ? 'Editing current counts'
                                : 'Counts look wrong? Adjust them',
                            style: typography.body.sm.copyWith(
                              color: colors.foreground,
                            ),
                          ),
                        ),
                        Text(
                          editCurrent.value ? 'Reset' : 'Edit',
                          style: typography.body.sm.copyWith(
                            fontWeight: FontWeight.w600,
                            color: colors.app.accentTone.onSubtle,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                AnimatedSize(
                  duration: Motion.medium,
                  curve: Curves.easeOutCubic,
                  child: editCurrent.value
                      ? Padding(
                          padding: const EdgeInsets.fromLTRB(
                            Space.lg,
                            0,
                            Space.lg,
                            Space.lg,
                          ),
                          child: Column(
                            children: [
                              CountStepper(
                                label: 'Attended',
                                value: curAttended.value,
                                onChanged: (v) {
                                  curTotal.value += v - curAttended.value;
                                  curAttended.value = v;
                                },
                              ),
                              const SizedBox(height: Space.md),
                              CountStepper(
                                label: 'Missed',
                                value: curTotal.value - curAttended.value,
                                onChanged: (v) =>
                                    curTotal.value = curAttended.value + v,
                              ),
                            ],
                          ),
                        )
                      : const SizedBox(width: double.infinity),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Figure extends StatelessWidget {
  const _Figure({
    required this.label,
    required this.value,
    required this.color,
    this.alignEnd = false,
    this.decimals = 1,
  });

  final String label;
  final double value;
  final Color color;
  final bool alignEnd;
  final int decimals;

  @override
  Widget build(BuildContext context) {
    final typography = context.theme.typography;
    return Column(
      crossAxisAlignment: alignEnd
          ? CrossAxisAlignment.end
          : CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: typography.body.xs.copyWith(
            fontSize: 10,
            fontWeight: FontWeight.w600,
            letterSpacing: 0.5,
            color: context.theme.colors.mutedForeground,
          ),
        ),
        TweenAnimationBuilder<double>(
          tween: Tween(end: value),
          duration: Motion.slow,
          curve: Curves.easeOutCubic,
          builder: (context, v, _) => Text(
            '${v.toStringAsFixed(decimals)}%',
            style: typography.display.xl2.copyWith(
              height: 1.1,
              fontWeight: FontWeight.w800,
              color: color,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ),
      ],
    );
  }
}

/// Upcoming classes as squares: attended first (green), then skipped (red).
class _PlanStrip extends StatelessWidget {
  const _PlanStrip({required this.attend, required this.skip});

  final int attend;
  final int skip;

  static const _max = 24;

  @override
  Widget build(BuildContext context) {
    final palette = context.theme.colors.app;
    final total = attend + skip;
    final shownAttend = total <= _max
        ? attend
        : (attend * _max / total).round();
    final shownSkip = total <= _max ? skip : _max - shownAttend;
    Widget square(Color color) => Container(
      width: 12,
      height: 12,
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(3),
      ),
    );
    return Wrap(
      spacing: 4,
      runSpacing: 4,
      children: [
        for (var i = 0; i < shownAttend; i++) square(palette.success.base),
        for (var i = 0; i < shownSkip; i++) square(palette.danger.base),
        if (total > _max)
          Text(
            ' $total classes',
            style: context.theme.typography.body.xs.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
      ],
    );
  }
}
