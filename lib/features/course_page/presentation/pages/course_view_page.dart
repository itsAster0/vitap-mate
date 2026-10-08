import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/utils/vtop_client_call.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/course_page/data/course_page_service.dart';
import 'package:vitapmate/features/course_page/domain/course_page_logic.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// Upcoming lectures shown before "show all".
const _upcomingShown = 3;

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
        log('course refresh failed: $e');
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
            padding: const EdgeInsets.all(Space.sm),
            children: const [SkeletonList(count: 5, height: 80)],
          ),
          error: (error, _) => ListView(
            children: [
              EmptyState(
                icon: FLucideIcons.cloudOff,
                title: "Couldn't load this course",
                message: vtopErrorMessage(error),
                action: FButton(
                  variant: FButtonVariant.outline,
                  mainAxisSize: MainAxisSize.min,
                  prefix: const Icon(FLucideIcons.rotateCw),
                  onPress: () =>
                      ref.invalidate(coursePageClassesProvider(course.id)),
                  child: const Text('Retry'),
                ),
              ),
            ],
          ),
          data: (_) => selected == null
              ? _SectionPicker(
                  course: course,
                  classes: list,
                  onPick: (c) => picked.value = c.classId,
                )
              : _LecturePlan(
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
  side: FLayout.btt,
  // Over the bottom navigation bar, so the keyboard inset is counted once.
  useRootNavigator: true,
  mainAxisMaxRatio: 0.8,
  builder: (context) {
    final colors = context.theme.colors;
    return Container(
      decoration: BoxDecoration(
        color: colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      ),
      child: SafeArea(
        top: false,
        child: ListView(
          shrinkWrap: true,
          padding: const EdgeInsets.fromLTRB(Space.lg, Space.md, Space.lg, 12),
          children: [
            const SheetHandle(),
            const SizedBox(height: Space.md),
            Text(
              'Sections',
              style: context.theme.typography.body.lg.copyWith(
                fontWeight: FontWeight.w700,
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
                padding: const EdgeInsets.only(bottom: Space.sm),
                child: _SectionTile(
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

class _SectionPicker extends StatelessWidget {
  const _SectionPicker({
    required this.course,
    required this.classes,
    required this.onPick,
  });

  final CoursePageCourse course;
  final List<CoursePageClass> classes;
  final ValueChanged<CoursePageClass> onPick;

  @override
  Widget build(BuildContext context) {
    if (classes.isEmpty) {
      return ListView(
        children: const [
          EmptyState(
            icon: FLucideIcons.bookOpen,
            title: 'No sections listed',
            message: 'VTOP has no classes for this course yet.',
          ),
        ],
      );
    }
    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(
        Space.sm,
        Space.sm,
        Space.sm,
        Space.xl,
      ),
      children: [
        _CourseTitle(course: course),
        const SectionHeader(title: 'Pick your section'),
        for (final (i, c) in classes.indexed)
          Padding(
            padding: const EdgeInsets.only(bottom: Space.sm),
            child: EnterFade(
              index: i,
              child: _SectionTile(
                section: c,
                isMine: false,
                selected: false,
                onPress: () => onPick(c),
              ),
            ),
          ),
      ],
    );
  }
}

class _CourseTitle extends StatelessWidget {
  const _CourseTitle({required this.course});

  final CoursePageCourse course;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Padding(
      padding: const EdgeInsets.fromLTRB(2, Space.xs, 2, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '${course.code} · ${courseTypeLabel(course.courseType)}',
            style: context.theme.typography.body.xs.copyWith(
              fontWeight: FontWeight.w600,
              letterSpacing: 0.3,
              color: colors.mutedForeground,
            ),
          ),
          Text(
            course.title,
            style: context.theme.typography.body.xl.copyWith(
              fontWeight: FontWeight.w700,
              color: colors.foreground,
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionTile extends StatelessWidget {
  const _SectionTile({
    required this.section,
    required this.isMine,
    required this.selected,
    required this.onPress,
  });

  final CoursePageClass section;
  final bool isMine;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Surface(
      onPress: onPress,
      borderColor: selected ? colors.app.accentTone.base : null,
      padding: const EdgeInsets.symmetric(
        horizontal: Space.md + 2,
        vertical: Space.md,
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Flexible(
                      child: Text(
                        section.faculty,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: context.theme.typography.body.sm.copyWith(
                          fontWeight: FontWeight.w700,
                          color: colors.foreground,
                        ),
                      ),
                    ),
                    if (isMine) ...[
                      const SizedBox(width: Space.sm),
                      ToneBadge(label: 'YOURS', tone: colors.app.accentTone),
                    ],
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  [
                    section.slot,
                    if (section.facultySchool.isNotEmpty) section.facultySchool,
                  ].join(' · '),
                  style: context.theme.typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          Icon(
            selected ? FLucideIcons.check : FLucideIcons.chevronRight,
            size: 16,
            color: selected
                ? colors.app.accentTone.base
                : colors.mutedForeground,
          ),
        ],
      ),
    );
  }
}

class _LecturePlan extends HookConsumerWidget {
  const _LecturePlan({
    required this.course,
    required this.section,
    required this.isMine,
    required this.sectionCount,
    required this.onSwitch,
  });

  final CoursePageCourse course;
  final CoursePageClass section;
  final bool isMine;
  final int sectionCount;
  final VoidCallback? onSwitch;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final detail = ref.watch(
      coursePageDetailProvider((section.erpId, section.classId)),
    );
    final withMaterialOnly = useState(false);
    final showAllUpcoming = useState(false);
    // Paths (or action ids) being fetched, for per-item spinners.
    final busy = useState<Set<String>>({});

    Future<void> run(String key, Future<void> Function() action) async {
      if (busy.value.contains(key)) return;
      busy.value = {...busy.value, key};
      try {
        await action();
      } catch (e) {
        log('course file failed: $e');
        if (context.mounted) {
          dispToast(context, "Couldn't get the file", vtopErrorMessage(e));
        }
      } finally {
        if (context.mounted) busy.value = {...busy.value}..remove(key);
      }
    }

    Future<void> download(Future<void> Function() start) async {
      await start();
      if (!context.mounted) return;
      dispToast(
        context,
        'Downloading',
        'Saving to Downloads. Tap the notification to open it.',
      );
    }

    final service = ref.read(coursePageServiceProvider);
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);

    final header = Surface(
      padding: const EdgeInsets.all(Space.md + 2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '${course.code} · ${courseTypeLabel(course.courseType)}',
            style: context.theme.typography.body.xs.copyWith(
              fontWeight: FontWeight.w600,
              letterSpacing: 0.3,
              color: colors.mutedForeground,
            ),
          ),
          Text(
            course.title,
            style: context.theme.typography.body.lg.copyWith(
              fontWeight: FontWeight.w700,
              color: colors.foreground,
            ),
          ),
          const SizedBox(height: Space.sm),
          Row(
            children: [
              Icon(FLucideIcons.user, size: 14, color: colors.mutedForeground),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  [
                    section.faculty,
                    if (section.facultySchool.isNotEmpty) section.facultySchool,
                  ].join(' · '),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: context.theme.typography.body.sm.copyWith(
                    color: colors.secondaryForeground,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Row(
            children: [
              Icon(FLucideIcons.clock, size: 14, color: colors.mutedForeground),
              const SizedBox(width: 6),
              Text(
                'Slot ${section.slot}',
                style: context.theme.typography.body.sm.copyWith(
                  color: colors.secondaryForeground,
                ),
              ),
              const SizedBox(width: Space.sm),
              Flexible(
                child: isMine
                    ? ToneBadge(
                        label: 'YOUR SECTION',
                        tone: colors.app.accentTone,
                      )
                    : ToneBadge.neutral(context, 'OTHER SECTION'),
              ),
            ],
          ),
          // On its own line: beside the slot it overflows narrow phones.
          if (onSwitch != null) ...[
            const SizedBox(height: Space.md),
            FButton(
              variant: FButtonVariant.outline,
              size: FButtonSizeVariant.sm,
              onPress: onSwitch,
              prefix: const Icon(FLucideIcons.arrowLeftRight),
              child: Text('See all $sectionCount sections'),
            ),
          ],
        ],
      ),
    );

    return detail.when(
      skipLoadingOnRefresh: true,
      loading: () => ListView(
        padding: const EdgeInsets.all(Space.sm),
        children: [
          header,
          const SizedBox(height: Space.lg),
          const SkeletonList(count: 6, height: 64),
        ],
      ),
      error: (error, _) => ListView(
        padding: const EdgeInsets.all(Space.sm),
        children: [
          header,
          EmptyState(
            icon: FLucideIcons.cloudOff,
            title: "Couldn't load the lecture plan",
            message: vtopErrorMessage(error),
            action: FButton(
              variant: FButtonVariant.outline,
              mainAxisSize: MainAxisSize.min,
              prefix: const Icon(FLucideIcons.rotateCw),
              onPress: () => ref.invalidate(
                coursePageDetailProvider((section.erpId, section.classId)),
              ),
              child: const Text('Retry'),
            ),
          ),
        ],
      ),
      data: (detail) {
        final lectures = detail.lectures
            .where((l) => !withMaterialOnly.value || l.materials.isNotEmpty)
            .toList();
        final upcoming = lectures.where((l) {
          final date = lectureDate(l);
          return date != null && !date.isBefore(today);
        }).toList();
        final covered = lectures.reversed.where((l) {
          final date = lectureDate(l);
          return date == null || date.isBefore(today);
        }).toList();
        final withMaterial = detail.lectures
            .where((l) => l.materials.isNotEmpty)
            .length;
        final shownUpcoming = showAllUpcoming.value
            ? upcoming
            : upcoming.take(_upcomingShown).toList();
        Widget lectureTile(CourseLecture lecture, {required bool planned}) =>
            _LectureTile(
              lecture: lecture,
              planned: planned,
              isNext: identical(lecture, upcoming.firstOrNull),
              busy: busy.value,
              onMaterial: (material) => run(
                material.path,
                () => download(() => service.file(material.path)),
              ),
            );

        return ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(
            Space.sm,
            Space.sm,
            Space.sm,
            Space.xl,
          ),
          children: [
            header,
            const SizedBox(height: Space.md),
            Wrap(
              spacing: Space.sm,
              runSpacing: Space.sm,
              children: [
                if (detail.syllabusPath.isNotEmpty)
                  _ActionChip(
                    icon: FLucideIcons.fileText,
                    label: 'Syllabus',
                    busy: busy.value.contains('syllabus'),
                    onPress: () => run(
                      'syllabus',
                      () => download(() => service.file(detail.syllabusPath)),
                    ),
                  ),
                if (detail.hasCoursePlan)
                  _ActionChip(
                    icon: FLucideIcons.sheet,
                    label: 'Course plan',
                    busy: busy.value.contains('plan'),
                    onPress: () => run(
                      'plan',
                      () => download(
                        () => service.coursePlan(
                          semesterId: detail.semesterId,
                          classId: section.classId,
                        ),
                      ),
                    ),
                  ),
                if (detail.generalMaterialsPath.isNotEmpty)
                  _ActionChip(
                    icon: FLucideIcons.folderArchive,
                    label: 'General material',
                    busy: busy.value.contains('general'),
                    onPress: () => run(
                      'general',
                      () => download(
                        () => service.file(detail.generalMaterialsPath),
                      ),
                    ),
                  ),
                if (detail.allMaterialsPath.isNotEmpty)
                  _ActionChip(
                    icon: FLucideIcons.download,
                    label: 'All material (zip)',
                    busy: busy.value.contains('all'),
                    onPress: () => run(
                      'all',
                      () =>
                          download(() => service.file(detail.allMaterialsPath)),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: Space.md),
            Segmented<bool>(
              segments: const [
                (false, 'All lectures'),
                (true, 'With material'),
              ],
              counts: [detail.lectures.length, withMaterial],
              value: withMaterialOnly.value,
              onChanged: (value) => withMaterialOnly.value = value,
            ),
            if (lectures.isEmpty)
              const EmptyState(
                icon: FLucideIcons.bookOpen,
                title: 'Nothing here yet',
                message: 'Lectures show up once the faculty posts them.',
              ),
            if (upcoming.isNotEmpty) ...[
              SectionHeader(
                title: 'Coming up',
                trailing: Text('${upcoming.length} planned'),
              ),
              for (final lecture in shownUpcoming)
                Padding(
                  padding: const EdgeInsets.only(bottom: Space.sm),
                  child: lectureTile(lecture, planned: true),
                ),
              if (upcoming.length > _upcomingShown)
                FButton(
                  variant: FButtonVariant.ghost,
                  size: FButtonSizeVariant.sm,
                  onPress: () => showAllUpcoming.value = !showAllUpcoming.value,
                  child: Text(
                    showAllUpcoming.value
                        ? 'Show fewer'
                        : 'Show all ${upcoming.length} planned',
                  ),
                ),
            ],
            if (covered.isNotEmpty) ...[
              SectionHeader(
                title: 'Covered',
                trailing: Text('${covered.length}'),
              ),
              for (final (i, lecture) in covered.indexed)
                Padding(
                  padding: const EdgeInsets.only(bottom: Space.sm),
                  child: EnterFade(
                    index: i.clamp(0, 8),
                    child: lectureTile(lecture, planned: false),
                  ),
                ),
            ],
          ],
        );
      },
    );
  }
}

class _ActionChip extends StatelessWidget {
  const _ActionChip({
    required this.icon,
    required this.label,
    required this.onPress,
    required this.busy,
  });

  final IconData icon;
  final String label;
  final VoidCallback onPress;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return PressScale(
      scale: 0.96,
      onPress: busy ? () {} : onPress,
      semanticsLabel: label,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: Space.md, vertical: 8),
        decoration: BoxDecoration(
          color: colors.card,
          borderRadius: BorderRadius.circular(Radii.md),
          border: Border.all(color: colors.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            busy
                ? const SizedBox.square(
                    dimension: 14,
                    child: FCircularProgress(),
                  )
                : Icon(icon, size: 14, color: colors.foreground),
            const SizedBox(width: 6),
            Text(
              label,
              style: context.theme.typography.body.xs.copyWith(
                fontWeight: FontWeight.w600,
                color: colors.foreground,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _LectureTile extends StatelessWidget {
  const _LectureTile({
    required this.lecture,
    required this.planned,
    required this.isNext,
    required this.busy,
    required this.onMaterial,
  });

  final CourseLecture lecture;
  final bool planned;
  final bool isNext;
  final Set<String> busy;
  final ValueChanged<CourseMaterial> onMaterial;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final date = lectureDate(lecture);
    final faded = planned && !isNext;
    return Surface(
      borderColor: isNext ? colors.app.accentTone.base : null,
      padding: const EdgeInsets.fromLTRB(
        Space.sm + 2,
        Space.sm + 2,
        Space.md,
        Space.sm + 2,
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 44,
            child: Column(
              children: [
                Text(
                  date == null ? '—' : '${date.day}',
                  style: context.theme.typography.body.lg.copyWith(
                    fontWeight: FontWeight.w700,
                    height: 1.1,
                    color: faded ? colors.mutedForeground : colors.foreground,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
                Text(
                  date == null
                      ? lecture.day
                      : DateFormat('MMM', 'en_US').format(date).toUpperCase(),
                  style: context.theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w600,
                    letterSpacing: 0.4,
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.sm),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      '#${lecture.serial} · ${lecture.day}',
                      style: context.theme.typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                    if (isNext) ...[
                      const SizedBox(width: Space.sm),
                      ToneBadge(
                        label: _isToday(date) ? 'TODAY' : 'NEXT',
                        tone: colors.app.accentTone,
                      ),
                    ] else if (planned) ...[
                      const SizedBox(width: Space.sm),
                      ToneBadge.neutral(context, 'PLANNED'),
                    ],
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  lecture.topic.isEmpty ? 'No topic given' : lecture.topic,
                  style: context.theme.typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: faded ? colors.mutedForeground : colors.foreground,
                  ),
                ),
                if (lecture.materials.isNotEmpty) ...[
                  const SizedBox(height: Space.sm),
                  Wrap(
                    spacing: Space.xs + 2,
                    runSpacing: Space.xs + 2,
                    children: [
                      for (final material in lecture.materials)
                        _MaterialChip(
                          label: materialLabel(material.label),
                          busy: busy.contains(material.path),
                          onPress: () => onMaterial(material),
                        ),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _MaterialChip extends StatelessWidget {
  const _MaterialChip({
    required this.label,
    required this.busy,
    required this.onPress,
  });

  final String label;
  final bool busy;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final tone = context.theme.colors.app.accentTone;
    return PressScale(
      scale: 0.95,
      onPress: busy ? () {} : onPress,
      semanticsLabel: label,
      child: Container(
        padding: const EdgeInsets.symmetric(
          horizontal: Space.sm + 2,
          vertical: 5,
        ),
        decoration: BoxDecoration(
          color: tone.subtle,
          borderRadius: BorderRadius.circular(Radii.pill),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            busy
                ? const SizedBox.square(
                    dimension: 12,
                    child: FCircularProgress(),
                  )
                : Icon(FLucideIcons.paperclip, size: 12, color: tone.onSubtle),
            const SizedBox(width: 5),
            Text(
              label,
              style: context.theme.typography.body.xs.copyWith(
                fontWeight: FontWeight.w600,
                color: tone.onSubtle,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

bool _isToday(DateTime? date) {
  if (date == null) return false;
  final now = DateTime.now();
  return date.year == now.year &&
      date.month == now.month &&
      date.day == now.day;
}
