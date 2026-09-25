import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_table.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// One course in the attendance list: ring, name, and what to do next.
class AttendanceCard extends ConsumerWidget {
  const AttendanceCard({super.key, required this.record, required this.index});

  final AttendanceRecord record;
  final int index;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final standing = AttendanceStanding.of(record);
    final tone = attendanceTone(
      context,
      safe: standing.isSafe,
      atEdge: standing.canSkip == 0,
    );
    final (code, name) = formateName(record.courseName);
    final isLab = record.islab();

    final pct = standing.displayPercent;
    return Surface(
      padding: EdgeInsets.zero,
      semanticsLabel:
          '$name, ${pct.round()} percent, ${standing.attended} of ${standing.total} attended, ${standing.advice}',
      onPress: () => showAttendanceDetails(context, record),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(Radii.lg - 1),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Container(width: 4, color: tone.base),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(
                    Space.md + 2,
                    Space.md + 2,
                    Space.md + 2,
                    Space.md,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  name.trim().isEmpty
                                      ? record.courseName
                                      : name.trim(),
                                  maxLines: 2,
                                  overflow: TextOverflow.ellipsis,
                                  style: typography.body.md.copyWith(
                                    height: 1.25,
                                    fontWeight: FontWeight.w600,
                                    color: colors.foreground,
                                  ),
                                ),
                                const SizedBox(height: Space.xs + 2),
                                Row(
                                  children: [
                                    CourseKindBadge(isLab: isLab),
                                    const SizedBox(width: Space.sm),
                                    Text(
                                      code.trim(),
                                      style: typography.body.xs.copyWith(
                                        color: colors.mutedForeground,
                                      ),
                                    ),
                                  ],
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(width: Space.md),
                          CountUp(
                            value: pct,
                            suffix: '%',
                            style: typography.body.lg.copyWith(
                              height: 1.2,
                              fontWeight: FontWeight.w500,
                              color: tone.base,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: Space.md),
                      SkipMeter(percent: pct, tone: tone),
                      const SizedBox(height: Space.md),
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.end,
                        children: [
                          _ClassCount(standing: standing),
                          const Spacer(),
                          _BufferDots(standing: standing, tone: tone),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// "22/24 attended".
class _ClassCount extends StatelessWidget {
  const _ClassCount({required this.standing});

  final AttendanceStanding standing;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Text(
      '${standing.attended}/${standing.total} attended',
      style: typography.body.sm.copyWith(
        color: colors.mutedForeground,
        fontFeatures: const [FontFeature.tabularFigures()],
      ),
    );
  }
}

/// One dot per class that can still be skipped (filled), or per class that
/// must be attended to recover (hollow), with the advice as a caption.
class _BufferDots extends StatelessWidget {
  const _BufferDots({required this.standing, required this.tone});

  final AttendanceStanding standing;
  final Tone tone;

  static const _maxDots = 6;

  @override
  Widget build(BuildContext context) {
    final typography = context.theme.typography;
    final recovering = !standing.isSafe;
    final count = recovering ? standing.mustAttend : standing.canSkip;
    final shown = count.clamp(0, _maxDots);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        if (shown > 0)
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              for (var i = 0; i < shown; i++)
                Container(
                  width: 8,
                  height: 8,
                  margin: const EdgeInsets.only(left: 3),
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: recovering ? null : tone.base,
                    border: recovering
                        ? Border.all(color: tone.base, width: 1.5)
                        : null,
                  ),
                ),
              if (count > _maxDots)
                Padding(
                  padding: const EdgeInsets.only(left: 4),
                  child: Text(
                    '+${count - _maxDots}',
                    style: typography.body.xs.copyWith(
                      fontWeight: FontWeight.w700,
                      color: tone.onSubtle,
                    ),
                  ),
                ),
            ],
          ),
        const SizedBox(height: 3),
        Text(
          standing.advice,
          style: typography.body.xs.copyWith(
            fontWeight: FontWeight.w500,
            color: standing.isSafe && standing.canSkip > 0
                ? context.theme.colors.mutedForeground
                : tone.onSubtle,
          ),
        ),
      ],
    );
  }
}

void showAttendanceDetails(BuildContext context, AttendanceRecord record) {
  showFSheet(
    context: context,
    side: FLayout.btt,
    // Above the tab bar, not inside the tab.
    useRootNavigator: true,
    mainAxisMaxRatio: 0.88,
    builder: (context) => SizedBox(
      height: MediaQuery.sizeOf(context).height * 0.88,
      child: AttendanceDetailSheet(record: record),
    ),
  );
}

/// Splits "CODE - Name - Type" into (code, name).
(String, String) formateName(String name) {
  final splitName = name.split("-");
  if (splitName.length < 2) return ("", name);
  var nName = splitName[1];
  if (splitName.length > 3) {
    nName += "-${splitName[2]}";
  }

  return (splitName[0], nName);
}
