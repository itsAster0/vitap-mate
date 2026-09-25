import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/utils/weightage_totals.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// Tone for a score percentage: strong, fine, or needs work.
Tone scoreTone(BuildContext context, double percent) {
  final palette = context.theme.colors.app;
  if (percent >= 75) return palette.success;
  if (percent >= 60) return palette.accentTone;
  return palette.warning;
}

/// One course's marks: weighted total up front, assessments on expand.
class MarksCard extends HookWidget {
  const MarksCard({super.key, required this.record});

  final MarksRecord record;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final expanded = useState(false);
    final totals = calculateWeightageTotals<MarksRecordEach>(
      record.marks,
      titleOf: (m) => m.markstitle,
      gainedOf: (m) => m.weightagemark,
      possibleOf: (m) => m.weightage,
    );
    final tone = scoreTone(context, totals.percentage);
    final hasMarks = totals.possible > 0;

    return Surface(
      padding: EdgeInsets.zero,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          PressScale(
            scale: 0.99,
            semanticsLabel:
                '${record.coursetitle}, ${totals.gained.toStringAsFixed(1)} of ${totals.possible.toStringAsFixed(0)}',
            onPress: () => expanded.value = !expanded.value,
            child: Padding(
              padding: const EdgeInsets.all(Space.md + 2),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              record.coursetitle,
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
                                CourseKindBadge(isLab: record.islab()),
                                const SizedBox(width: Space.sm),
                                Text(
                                  record.coursecode,
                                  style: typography.body.xs.copyWith(
                                    color: colors.mutedForeground,
                                  ),
                                ),
                              ],
                            ),
                            if (record.faculity.trim().isNotEmpty) ...[
                              const SizedBox(height: Space.xs + 2),
                              FacultyLine(name: record.faculity),
                            ],
                          ],
                        ),
                      ),
                      const SizedBox(width: Space.md),
                      Column(
                        crossAxisAlignment: CrossAxisAlignment.end,
                        children: [
                          Text.rich(
                            TextSpan(
                              children: [
                                TextSpan(
                                  text: _fmt(totals.gained),
                                  style: TextStyle(color: colors.foreground),
                                ),
                                TextSpan(
                                  text: ' / ${_fmt(totals.possible)}',
                                  style: TextStyle(
                                    color: colors.mutedForeground,
                                  ),
                                ),
                              ],
                            ),
                            style: typography.body.lg.copyWith(
                              height: 1.2,
                              fontWeight: FontWeight.w500,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                          if (hasMarks)
                            Text(
                              '${totals.percentage.toStringAsFixed(0)}%',
                              style: typography.body.xs.copyWith(
                                fontWeight: FontWeight.w500,
                                color: tone.base,
                              ),
                            ),
                        ],
                      ),
                    ],
                  ),
                  const SizedBox(height: Space.md),
                  Row(
                    children: [
                      Expanded(
                        child: ProgressBar(
                          value: hasMarks ? totals.percentage / 100 : 0,
                          color: tone.base,
                        ),
                      ),
                      const SizedBox(width: Space.md),
                      Text(
                        '${record.marks.length} ${record.marks.length == 1 ? 'assessment' : 'assessments'}',
                        style: typography.body.xs.copyWith(
                          color: colors.mutedForeground,
                        ),
                      ),
                      const SizedBox(width: Space.xs),
                      AnimatedRotation(
                        turns: expanded.value ? 0.5 : 0,
                        duration: Motion.medium,
                        curve: Curves.easeOutCubic,
                        child: Icon(
                          FLucideIcons.chevronDown,
                          size: 16,
                          color: colors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          AnimatedSize(
            duration: Motion.medium,
            curve: Curves.easeOutCubic,
            alignment: Alignment.topCenter,
            child: expanded.value
                ? Column(
                    children: [
                      Container(height: 1, color: colors.border),
                      for (final (i, mark) in record.marks.indexed) ...[
                        if (i > 0)
                          Container(
                            height: 1,
                            margin: const EdgeInsets.symmetric(
                              horizontal: Space.md + 2,
                            ),
                            color: colors.border.withValues(alpha: 0.6),
                          ),
                        _AssessmentRow(mark: mark),
                      ],
                    ],
                  )
                : const SizedBox(width: double.infinity),
          ),
        ],
      ),
    );
  }
}

class _AssessmentRow extends StatelessWidget {
  const _AssessmentRow({required this.mark});

  final MarksRecordEach mark;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final scored = double.tryParse(mark.scoredmark.trim()) ?? 0;
    final max = double.tryParse(mark.maxmarks.trim()) ?? 0;
    final percent = max > 0 ? scored / max * 100 : 0.0;
    final absent = mark.status.trim().toLowerCase() == 'absent';
    final tone = absent ? colors.app.danger : scoreTone(context, percent);

    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.md + 2,
        vertical: Space.md,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  mark.markstitle,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
              ),
              if (absent) ...[
                ToneBadge(label: 'ABSENT', tone: colors.app.danger),
                const SizedBox(width: Space.sm),
              ],
              Text.rich(
                TextSpan(
                  children: [
                    TextSpan(
                      text: mark.scoredmark,
                      style: const TextStyle(fontWeight: FontWeight.w700),
                    ),
                    TextSpan(
                      text: ' / ${mark.maxmarks}',
                      style: TextStyle(color: colors.mutedForeground),
                    ),
                  ],
                ),
                style: typography.body.sm.copyWith(
                  color: colors.foreground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.sm),
          Row(
            children: [
              Expanded(
                child: ProgressBar(
                  value: percent / 100,
                  color: tone.base,
                  height: 4,
                ),
              ),
              const SizedBox(width: Space.md),
              Text(
                'Weighted ${mark.weightagemark} / ${mark.weightage}',
                style: typography.body.xs.copyWith(
                  color: colors.mutedForeground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ],
          ),
          if (mark.remark.trim().isNotEmpty) ...[
            const SizedBox(height: Space.xs),
            Text(
              mark.remark.trim(),
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
          ],
        ],
      ),
    );
  }
}

String _fmt(double value) => value == value.roundToDouble()
    ? value.toStringAsFixed(0)
    : value.toStringAsFixed(1);
