import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/ui/grade_badge.dart';
import 'package:vitapmate/features/grades/grades.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/features/grades/widgets/grade_details_panel.dart';

/// One course's grade; tap to expand its mark breakdown.
class GradeCard extends HookConsumerWidget {
  const GradeCard({super.key, required this.course});

  final GradeCourseRecord course;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final controller = useAnimationController(
      duration: const Duration(milliseconds: 250),
    );
    final animation = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeInOut),
    );
    final expanded = useState(false);
    final state = ref.watch(gradesProvider).value;
    final c = course;

    Future<void> loadDetails({bool force = false}) =>
        ref.read(gradesProvider.notifier).loadDetails(c.courseId, force: force);

    Future<void> toggle() async {
      expanded.value = !expanded.value;
      if (!expanded.value) {
        controller.reverse();
        return;
      }
      controller.forward();
      final s = ref.read(gradesProvider).value;
      final detail = s?.detailsByCourseId[c.courseId];
      final loading = s?.loadingDetailsFor.contains(c.courseId) ?? false;
      // A "#" in a cut-off means VTOP sent a placeholder: fetch it again.
      final hasMarkerRange =
          detail?.gradeRanges.any((r) => r.range.contains('#')) ?? false;
      if (loading || (detail != null && !hasMarkerRange)) return;
      try {
        await loadDetails(force: detail != null);
      } catch (e) {
        Log.info('features.grades', "$e");
      }
    }

    return Surface(
      padding: .zero,
      child: Column(
        children: [
          PressScale(
            scale: 0.99,
            semanticsLabel: '${c.courseTitle}, grade ${c.grade}',
            onPress: toggle,
            child: Padding(
              padding: const .all(Space.md + 2),
              child: Row(
                children: [
                  GradeBadge(grade: c.grade),
                  const SizedBox(width: Space.md),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: .start,
                      children: [
                        Text(
                          c.courseTitle,
                          maxLines: 2,
                          overflow: .ellipsis,
                          style: typography.body.md.copyWith(
                            height: 1.25,
                            fontWeight: .w600,
                            color: colors.foreground,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${c.courseCode} · ${c.courseType}',
                          maxLines: 1,
                          overflow: .ellipsis,
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: Space.md),
                  Column(
                    crossAxisAlignment: .end,
                    children: [
                      Text(
                        c.grandTotal,
                        style: typography.body.lg.nums(
                          fontWeight: .w500,
                          color: colors.foreground,
                        ),
                      ),
                      Text(
                        'total',
                        style: typography.body.xs.copyWith(
                          color: colors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(width: Space.xs),
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
            ),
          ),
          SizeTransition(
            sizeFactor: animation,
            alignment: .topCenter,
            child: GradeDetailsPanel(
              course: c,
              detail: state?.detailsByCourseId[c.courseId],
              loading: state?.loadingDetailsFor.contains(c.courseId) ?? false,
              onRefresh: () async {
                try {
                  await loadDetails(force: true);
                } catch (e) {
                  Log.info('features.grades', "$e");
                  if (context.mounted) disCommonToast(context, e);
                }
              },
            ),
          ),
        ],
      ),
    );
  }
}
