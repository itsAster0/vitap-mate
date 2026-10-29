import 'package:vitapmate/observability/log.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/vtop/vtop_client_call.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/course_page/course_page_service.dart';
import 'package:vitapmate/features/course_page/logic/course_page_logic.dart';
import 'package:vitapmate/features/course_page/widgets/lecture_tile.dart';
import 'package:vitapmate/features/course_page/widgets/section_picker.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// Upcoming lectures shown before "show all".
const _upcomingShown = 3;

/// A section's header, file downloads and lectures, upcoming then covered.
class LecturePlan extends HookConsumerWidget {
  const LecturePlan({
    super.key,
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

    /// Hands a file to the system downloader, with a spinner on [key].
    Future<void> download(String key, Future<void> Function() start) async {
      if (busy.value.contains(key)) return;
      busy.value = {...busy.value, key};
      try {
        await start();
        if (context.mounted) {
          dispToast(
            context,
            'Downloading',
            'Saving to Downloads. Tap the notification to open it.',
          );
        }
      } catch (e) {
        Log.info('features.course_page', 'course file failed: $e');
        if (context.mounted) {
          dispToast(context, "Couldn't get the file", vtopErrorMessage(e));
        }
      } finally {
        if (context.mounted) busy.value = {...busy.value}..remove(key);
      }
    }

    final service = ref.read(coursePageServiceProvider);
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);

    final header = Surface(
      padding: const .all(Space.md + 2),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          CourseTitle(course: course),
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
                  overflow: .ellipsis,
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
              variant: .outline,
              size: .sm,
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
        padding: const .all(Space.sm),
        children: [
          header,
          const SizedBox(height: Space.lg),
          const SkeletonList(count: 6, height: 64),
        ],
      ),
      error: (error, _) => ListView(
        padding: const .all(Space.sm),
        children: [
          header,
          EmptyState(
            icon: FLucideIcons.cloudOff,
            title: "Couldn't load the lecture plan",
            message: vtopErrorMessage(error),
            action: FButton(
              variant: .outline,
              mainAxisSize: .min,
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
        bool ahead(CourseLecture l) => lectureDate(l)?.isBefore(today) == false;
        final upcoming = lectures.where(ahead).toList();
        final covered = lectures.reversed.where((l) => !ahead(l)).toList();
        final withMaterial = detail.lectures
            .where((l) => l.materials.isNotEmpty)
            .length;
        final shownUpcoming = showAllUpcoming.value
            ? upcoming
            : upcoming.take(_upcomingShown).toList();
        Widget lectureTile(CourseLecture lecture, {required bool planned}) =>
            LectureTile(
              lecture: lecture,
              planned: planned,
              isNext: identical(lecture, upcoming.firstOrNull),
              busy: busy.value,
              onMaterial: (material) =>
                  download(material.path, () => service.file(material.path)),
            );

        return ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.xl),
          children: [
            header,
            const SizedBox(height: Space.md),
            Wrap(
              spacing: Space.sm,
              runSpacing: Space.sm,
              children: [
                for (final (key, icon, label, start) in [
                  if (detail.syllabusPath.isNotEmpty)
                    (
                      'syllabus',
                      FLucideIcons.fileText,
                      'Syllabus',
                      () => service.file(detail.syllabusPath),
                    ),
                  if (detail.hasCoursePlan)
                    (
                      'plan',
                      FLucideIcons.sheet,
                      'Course plan',
                      () => service.coursePlan(
                        semesterId: detail.semesterId,
                        classId: section.classId,
                      ),
                    ),
                  if (detail.generalMaterialsPath.isNotEmpty)
                    (
                      'general',
                      FLucideIcons.folderArchive,
                      'General material',
                      () => service.file(detail.generalMaterialsPath),
                    ),
                  if (detail.allMaterialsPath.isNotEmpty)
                    (
                      'all',
                      FLucideIcons.download,
                      'All material (zip)',
                      () => service.file(detail.allMaterialsPath),
                    ),
                ])
                  _ActionChip(
                    icon: icon,
                    label: label,
                    busy: busy.value.contains(key),
                    onPress: () => download(key, start),
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
                  padding: const .only(bottom: Space.sm),
                  child: lectureTile(lecture, planned: true),
                ),
              if (upcoming.length > _upcomingShown)
                FButton(
                  variant: .ghost,
                  size: .sm,
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
                  padding: const .only(bottom: Space.sm),
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
        padding: const .symmetric(horizontal: Space.md, vertical: 8),
        decoration: BoxDecoration(
          color: colors.card,
          borderRadius: .circular(Radii.md),
          border: Border.all(color: colors.border),
        ),
        child: Row(
          mainAxisSize: .min,
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
                fontWeight: .w600,
                color: colors.foreground,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
