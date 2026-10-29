import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/grade_history/grade_history.dart';
import 'package:vitapmate/features/grades/grades.dart';
import 'package:vitapmate/vtop/semesters.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/grades/widgets/grade_card.dart';
import 'package:vitapmate/features/grades/widgets/semester_summary.dart';

class GradesPage extends HookConsumerWidget {
  const GradesPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(gradesProvider);
    final semAsync = ref.watch(semesterIdProvider);
    // Stable keys per course so the summary bar can scroll to a card.
    final cardKeys = useMemoized(() => <String, GlobalKey>{});
    final semesters = switch (state.value?.semesters) {
      final sems? when sems.isNotEmpty => sems,
      _ => semAsync.value?.semesters ?? const <SemesterInfo>[],
    };

    Future<void> refresh() async {
      try {
        await ref.read(semesterIdProvider.future);
        await ref.read(gradesProvider.notifier).refresh();
      } catch (e) {
        Log.info('features.grades', "$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        try {
          await ref.read(gradesProvider.future);
          await ref.read(gradesProvider.notifier).refresh();
        } catch (e, st) {
          Log.warn('features.grades', 'auto refresh failed: $e\n$st');
        }
      });
      return null;
    }, const []);

    final creditsByCode = {
      for (final r
          in ref.watch(gradeHistoryProvider).value?.records ??
              const <GradeHistoryRecord>[])
        r.courseCode.trim().toUpperCase(): double.tryParse(r.credits.trim()),
    };

    return ScreenRefresh(
      onRefresh: refresh,
      tasks: const ['vtop_grades', 'vtop_grade_details'],
      child: RefreshIndicator(
        backgroundColor: context.theme.colors.background,
        color: context.theme.colors.foreground,
        onRefresh: refresh,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.lg),
          children: [
            if (semesters.isEmpty && semAsync.isLoading)
              const Skeleton(height: 36, radius: Radii.pill)
            else if (semesters.isEmpty && semAsync.hasError)
              FButton(
                variant: .outline,
                prefix: const Icon(FLucideIcons.rotateCw),
                onPress: () => ref.invalidate(semesterIdProvider),
                child: const Text("Couldn't load semesters. Retry"),
              )
            else
              SemesterChips(
                semesters: semesters,
                selectedId:
                    state.value?.selectedSemesterId ??
                    semesters.firstOrNull?.id ??
                    "",
                onSelect: (id) async {
                  try {
                    await ref.read(gradesProvider.notifier).selectSemester(id);
                  } catch (e) {
                    Log.info('features.grades', "$e");
                  }
                },
              ),
            const SizedBox(height: Space.md),
            AnimatedSwitcher(
              duration: Motion.medium,
              child: state.when(
                skipLoadingOnRefresh: true,
                loading: () => const Column(
                  key: ValueKey('loading'),
                  children: [
                    Skeleton(height: 96, radius: Radii.lg),
                    SizedBox(height: Space.md),
                    SkeletonList(count: 5, height: 76),
                  ],
                ),
                error: (e, _) => EmptyState(
                  key: const ValueKey('error'),
                  icon: FLucideIcons.cloudAlert,
                  title: "Couldn't load grades",
                  message: commonErrorMessage(e),
                ),
                data: (data) {
                  int serial(GradeCourseRecord c) =>
                      int.tryParse(c.serial) ?? 0;
                  final sorted = [...data.gradeView.courses]
                    ..sort((a, b) => serial(a).compareTo(serial(b)));
                  return Column(
                    key: ValueKey('data_${data.selectedSemesterId}'),
                    crossAxisAlignment: .stretch,
                    children: [
                      if (sorted.isEmpty)
                        const EmptyState(
                          icon: FLucideIcons.school,
                          title: 'No grades for this semester',
                          message: 'Grades appear after results are declared.',
                        )
                      else ...[
                        SemesterSummary(
                          courses: sorted,
                          creditsByCode: creditsByCode,
                          onCourse: (c) {
                            final ctx = cardKeys[c.courseId]?.currentContext;
                            if (ctx == null) return;
                            Scrollable.ensureVisible(
                              ctx,
                              duration: Motion.slow,
                              curve: Curves.easeOutCubic,
                              alignment: 0.1,
                            );
                          },
                        ),
                        const SizedBox(height: Space.md),
                        for (final (i, c) in sorted.indexed)
                          Padding(
                            padding: const .only(bottom: Space.sm),
                            child: EnterFade(
                              index: i,
                              child: KeyedSubtree(
                                key: cardKeys.putIfAbsent(
                                  c.courseId,
                                  GlobalKey.new,
                                ),
                                child: GradeCard(course: c),
                              ),
                            ),
                          ),
                      ],
                      DataUpdatedFooter(
                        updateTime: data.gradeView.updateTime.toInt(),
                      ),
                    ],
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );
  }
}
