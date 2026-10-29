import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/vtop/vtop_client_call.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/course_page/course_page_service.dart';
import 'package:vitapmate/features/course_page/logic/course_page_logic.dart';
import 'package:vitapmate/features/course_page/widgets/lecture_plan.dart';
import 'package:vitapmate/features/course_page/widgets/section_picker.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// One course part: the student's section's lecture plan, with the other
/// sections a tap away.
class CourseViewPage extends HookConsumerWidget {
  const CourseViewPage({super.key, required this.course});

  final CoursePageCourse course;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final classes = ref.watch(coursePageClassesProvider(course.id));
    final timetable = ref.watch(timetableProvider).value;
    // Null until the student picks a section; then it wins over the guess.
    final picked = useState<String?>(null);

    final list = classes.value?.classes ?? const <CoursePageClass>[];
    final mine = myClass(list, timetable);
    final selected =
        list.where((c) => c.classId == picked.value).firstOrNull ??
        mine ??
        (list.length == 1 ? list.single : null);

    Future<void> refresh() async {
      ref.invalidate(coursePageClassesProvider(course.id));
      if (selected != null) {
        ref.invalidate(
          coursePageDetailProvider((selected.erpId, selected.classId)),
        );
      }
      try {
        await ref.read(coursePageClassesProvider(course.id).future);
      } catch (e) {
        Log.info('features.course_page', 'course refresh failed: $e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    Future<void> chooseSection() async {
      final choice = await _showSections(context, list, mine, selected);
      if (choice != null) picked.value = choice.classId;
    }

    return ScreenRefresh(
      onRefresh: refresh,
      child: RefreshIndicator(
        onRefresh: refresh,
        backgroundColor: colors.background,
        color: colors.foreground,
        child: classes.when(
          skipLoadingOnRefresh: true,
          loading: () => ListView(
            padding: const .all(Space.sm),
            children: const [SkeletonList(count: 5, height: 80)],
          ),
          error: (error, _) => ListView(
            children: [
              EmptyState(
                icon: FLucideIcons.cloudOff,
                title: "Couldn't load this course",
                message: vtopErrorMessage(error),
                action: FButton(
                  variant: .outline,
                  mainAxisSize: .min,
                  prefix: const Icon(FLucideIcons.rotateCw),
                  onPress: () =>
                      ref.invalidate(coursePageClassesProvider(course.id)),
                  child: const Text('Retry'),
                ),
              ),
            ],
          ),
          data: (_) => selected == null
              ? SectionPicker(
                  course: course,
                  classes: list,
                  onPick: (c) => picked.value = c.classId,
                )
              : LecturePlan(
                  course: course,
                  section: selected,
                  isMine: selected.classId == mine?.classId,
                  sectionCount: list.length,
                  onSwitch: list.length > 1 ? chooseSection : null,
                ),
        ),
      ),
    );
  }
}

Future<CoursePageClass?> _showSections(
  BuildContext context,
  List<CoursePageClass> classes,
  CoursePageClass? mine,
  CoursePageClass? selected,
) => showFSheet<CoursePageClass>(
  context: context,
  side: .btt,
  // Over the bottom navigation bar, so the keyboard inset is counted once.
  useRootNavigator: true,
  mainAxisMaxRatio: 0.8,
  builder: (context) {
    final colors = context.theme.colors;
    return Container(
      decoration: BoxDecoration(
        color: colors.background,
        borderRadius: const .vertical(top: Radius.circular(10)),
      ),
      child: SafeArea(
        top: false,
        child: ListView(
          shrinkWrap: true,
          padding: const .fromLTRB(Space.lg, Space.md, Space.lg, 12),
          children: [
            const SheetHandle(),
            const SizedBox(height: Space.md),
            Text(
              'Sections',
              style: context.theme.typography.body.lg.copyWith(
                fontWeight: .w700,
                color: colors.foreground,
              ),
            ),
            Text(
              "Any section's lecture plan and material is open to you.",
              style: context.theme.typography.body.xs.copyWith(
                color: colors.mutedForeground,
              ),
            ),
            const SizedBox(height: Space.md),
            for (final c in classes)
              Padding(
                padding: const .only(bottom: Space.sm),
                child: SectionTile(
                  section: c,
                  isMine: c.classId == mine?.classId,
                  selected: c.classId == selected?.classId,
                  onPress: () => Navigator.of(context).pop(c),
                ),
              ),
          ],
        ),
      ),
    );
  },
);
