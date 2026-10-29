import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/ui/grade_badge.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/features/grades/logic/grade_details.dart';

/// A course's grade cut-offs and marks by section, with a refresh button.
class GradeDetailsPanel extends StatelessWidget {
  const GradeDetailsPanel({
    super.key,
    required this.course,
    required this.detail,
    required this.loading,
    required this.onRefresh,
  });

  final GradeCourseRecord course;
  final GradeDetailsData? detail;
  final bool loading;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final detail = this.detail;
    if (detail == null) {
      return Padding(
        padding: const .all(Space.md),
        child: loading
            ? const Column(
                children: [
                  Skeleton(height: 28, radius: Radii.md),
                  SizedBox(height: Space.sm),
                  Skeleton(height: 44, radius: Radii.md),
                  SizedBox(height: Space.sm),
                  Skeleton(height: 44, radius: Radii.md),
                ],
              )
            : Row(
                children: [
                  Expanded(
                    child: Text(
                      "Couldn't load the mark breakdown.",
                      style: typography.body.sm.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                  ),
                  FButton(
                    variant: .outline,
                    size: .sm,
                    mainAxisSize: .min,
                    onPress: onRefresh,
                    child: const Text('Try again'),
                  ),
                ],
              ),
      );
    }

    final ranges = [
      for (final r in detail.gradeRanges)
        if (r.range.replaceAll('#', '').trim().isNotEmpty)
          (r.grade.trim().toUpperCase(), shortRange(r.range)),
    ];
    final gradingType = course.gradingType.trim();

    return Padding(
      padding: const .fromLTRB(Space.md + 2, Space.xs, Space.md + 2, Space.md),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          if (ranges.isNotEmpty) ...[
            _Label(
              gradingType.isEmpty
                  ? 'GRADE CUT-OFFS'
                  : 'GRADE CUT-OFFS · $gradingType',
            ),
            const SizedBox(height: Space.sm),
            Wrap(
              spacing: Space.xs + 2,
              runSpacing: Space.xs + 2,
              children: [
                for (final (g, range) in ranges)
                  _CutoffChip(
                    grade: g,
                    range: range,
                    achieved: g == course.grade.trim().toUpperCase(),
                  ),
              ],
            ),
          ],
          for (final section in gradeSections(detail)) ...[
            const SizedBox(height: Space.lg),
            Row(
              children: [
                Expanded(child: _Label(section.title.toUpperCase())),
                if (section.total.isNotEmpty)
                  Text(
                    '${section.total} weighted',
                    style: typography.body.xs.copyWith(
                      fontWeight: .w600,
                      color: colors.foreground,
                    ),
                  ),
              ],
            ),
            const SizedBox(height: Space.xs),
            for (final (i, m) in section.marks.indexed) ...[
              if (i > 0)
                Container(
                  height: 1,
                  color: colors.border.withValues(alpha: 0.6),
                ),
              _MarkRow(mark: m),
            ],
          ],
          // Refresh just this course's breakdown.
          const SizedBox(height: Space.sm),
          Row(
            children: [
              Expanded(
                child: Text(
                  loading ? 'Updating from VTOP…' : 'Breakdown from VTOP',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ),
              FButton(
                variant: .ghost,
                size: .sm,
                mainAxisSize: .min,
                onPress: loading ? null : onRefresh,
                prefix: loading
                    ? const SizedBox.square(
                        dimension: 14,
                        child: FCircularProgress(size: .sm),
                      )
                    : const Icon(FLucideIcons.refreshCw),
                child: const Text('Refresh'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _Label extends StatelessWidget {
  const _Label(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => Text(
    text,
    style: context.theme.typography.body.xs.copyWith(
      fontSize: 10.5,
      fontWeight: .w600,
      letterSpacing: 0.5,
      color: context.theme.colors.mutedForeground,
    ),
  );
}

class _CutoffChip extends StatelessWidget {
  const _CutoffChip({
    required this.grade,
    required this.range,
    required this.achieved,
  });

  final String grade;
  final String range;
  final bool achieved;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final tone = gradeTone(context, grade);
    return Container(
      padding: const .symmetric(horizontal: Space.sm, vertical: 4),
      decoration: BoxDecoration(
        color: achieved ? tone.subtle : const Color(0x00000000),
        borderRadius: .circular(Radii.sm),
        border: .all(color: achieved ? tone.base : colors.border),
      ),
      child: Text.rich(
        TextSpan(
          children: [
            TextSpan(
              text: '$grade ',
              style: TextStyle(
                fontWeight: .w700,
                color: achieved ? tone.onSubtle : colors.foreground,
              ),
            ),
            TextSpan(
              text: range,
              style: TextStyle(
                color: achieved ? tone.onSubtle : colors.mutedForeground,
              ),
            ),
          ],
        ),
        style: context.theme.typography.body.xs.nums(),
      ),
    );
  }
}

class _MarkRow extends StatelessWidget {
  const _MarkRow({required this.mark});

  final GradeDetailMark mark;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final scored = double.tryParse(mark.scoredMark.trim()) ?? 0;
    final max = double.tryParse(mark.maxMark.trim()) ?? 0;
    final pct = max > 0 ? scored / max * 100 : 0.0;
    final tone = pct >= 75
        ? colors.app.success
        : pct >= 60
        ? colors.app.accentTone
        : colors.app.warning;

    return Padding(
      padding: const .symmetric(vertical: Space.sm),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  mark.markTitle,
                  maxLines: 2,
                  overflow: .ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: .w500,
                    color: colors.foreground,
                  ),
                ),
              ),
              Text.rich(
                TextSpan(
                  children: [
                    TextSpan(text: mark.scoredMark.trim()),
                    TextSpan(
                      text: ' / ${mark.maxMark.trim()}',
                      style: TextStyle(color: colors.mutedForeground),
                    ),
                  ],
                ),
                style: typography.body.sm.nums(
                  fontWeight: .w500,
                  color: colors.foreground,
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.xs + 2),
          Row(
            children: [
              Expanded(
                child: ProgressBar(
                  value: pct / 100,
                  color: tone.base,
                  height: 4,
                ),
              ),
              const SizedBox(width: Space.md),
              Text(
                '${mark.weightageMark.trim()} of ${mark.weightage.trim()}',
                style: typography.body.xs.nums(color: colors.mutedForeground),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
