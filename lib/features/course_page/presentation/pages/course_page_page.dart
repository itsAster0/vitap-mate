import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/course_page/data/course_page_service.dart';
import 'package:vitapmate/features/course_page/domain/course_page_logic.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// The student's courses this semester; each part (theory, lab...) opens
/// its lecture plan.
class CoursePagePage extends ConsumerWidget {
  const CoursePagePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final data = ref.watch(coursePageCoursesProvider);

    Future<void> refresh() async {
      ref.invalidate(coursePageCoursesProvider);
      try {
        await ref.read(coursePageCoursesProvider.future);
      } catch (e) {
        log('course page refresh failed: $e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    void open(CoursePageCourse course) =>
        GoRouter.of(context).pushNamed(Paths.coursePageCourse, extra: course);

    return ScreenRefresh(
      onRefresh: refresh,
      child: RefreshIndicator(
        onRefresh: refresh,
        backgroundColor: colors.background,
        color: colors.foreground,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(
            Space.sm,
            Space.sm,
            Space.sm,
            Space.xl,
          ),
          children: data.when(
            skipLoadingOnRefresh: true,
            loading: () => const [SkeletonList(count: 6, height: 84)],
            error: (error, _) => [
              EmptyState(
                icon: FLucideIcons.cloudOff,
                title: "Couldn't load your courses",
                message:
                    'VTOP did not answer. Check your connection and retry.',
                action: FButton(
                  variant: FButtonVariant.outline,
                  prefix: const Icon(FLucideIcons.rotateCw),
                  mainAxisSize: MainAxisSize.min,
                  onPress: () => ref.invalidate(coursePageCoursesProvider),
                  child: const Text('Retry'),
                ),
              ),
            ],
            data: (data) {
              final groups = groupCourses(data.courses);
              if (groups.isEmpty) {
                return const [
                  EmptyState(
                    icon: FLucideIcons.bookOpen,
                    title: 'No courses this semester',
                    message: 'Courses you register for show up here.',
                  ),
                ];
              }
              return [
                Padding(
                  padding: const EdgeInsets.fromLTRB(2, 0, 2, Space.sm),
                  child: Text(
                    'Lecture plans and material from your faculty.',
                    style: context.theme.typography.body.xs.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                ),
                for (final (i, group) in groups.indexed)
                  Padding(
                    padding: const EdgeInsets.only(bottom: Space.sm),
                    child: EnterFade(
                      index: i,
                      child: _CourseCard(group: group, onOpen: open),
                    ),
                  ),
              ];
            },
          ),
        ),
      ),
    );
  }
}

class _CourseCard extends StatelessWidget {
  const _CourseCard({required this.group, required this.onOpen});

  final CourseGroup group;
  final ValueChanged<CoursePageCourse> onOpen;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Surface(
      semanticsLabel: '${group.code} ${group.title}',
      padding: const EdgeInsets.all(Space.md + 2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            group.code,
            style: context.theme.typography.body.xs.copyWith(
              fontWeight: FontWeight.w600,
              letterSpacing: 0.3,
              color: colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            group.title,
            style: context.theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
              color: colors.foreground,
            ),
          ),
          const SizedBox(height: Space.md),
          // One button per part, sharing the card's full width.
          Row(
            children: [
              for (final (i, part) in group.parts.indexed) ...[
                if (i > 0) const SizedBox(width: Space.sm),
                Expanded(
                  child: _PartButton(
                    label: courseTypeLabel(part.courseType),
                    lab: isLabType(part.courseType),
                    onPress: () => onOpen(part),
                  ),
                ),
              ],
            ],
          ),
        ],
      ),
    );
  }
}

class _PartButton extends StatelessWidget {
  const _PartButton({
    required this.label,
    required this.lab,
    required this.onPress,
  });

  final String label;
  final bool lab;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final tone = lab ? colors.app.lab : colors.app.accentTone;
    return PressScale(
      scale: 0.97,
      onPress: onPress,
      semanticsLabel: label,
      child: Container(
        height: 44,
        decoration: BoxDecoration(
          color: tone.subtle,
          borderRadius: BorderRadius.circular(Radii.md),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              lab ? FLucideIcons.flaskConical : FLucideIcons.bookOpen,
              size: 16,
              color: tone.onSubtle,
            ),
            const SizedBox(width: Space.sm),
            Text(
              label,
              style: context.theme.typography.body.sm.copyWith(
                fontWeight: FontWeight.w600,
                color: tone.onSubtle,
              ),
            ),
            const SizedBox(width: Space.xs),
            Icon(FLucideIcons.chevronRight, size: 15, color: tone.onSubtle),
          ],
        ),
      ),
    );
  }
}
