import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/course_page/logic/course_page_logic.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// The section list shown until the student's section is known.
class SectionPicker extends StatelessWidget {
  const SectionPicker({
    super.key,
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
      padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.xl),
      children: [
        Padding(
          padding: const .fromLTRB(2, Space.xs, 2, 0),
          child: CourseTitle(course: course, large: true),
        ),
        const SectionHeader(title: 'Pick your section'),
        for (final (i, c) in classes.indexed)
          Padding(
            padding: const .only(bottom: Space.sm),
            child: EnterFade(
              index: i,
              child: SectionTile(
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

/// The course code and part over its title ([large] above the section list).
class CourseTitle extends StatelessWidget {
  const CourseTitle({super.key, required this.course, this.large = false});

  final CoursePageCourse course;
  final bool large;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final body = context.theme.typography.body;
    return Column(
      crossAxisAlignment: .start,
      children: [
        Text(
          '${course.code} · ${courseTypeLabel(course.courseType)}',
          style: body.xs.copyWith(
            fontWeight: .w600,
            letterSpacing: 0.3,
            color: colors.mutedForeground,
          ),
        ),
        Text(
          course.title,
          style: (large ? body.xl : body.lg).copyWith(
            fontWeight: .w700,
            color: colors.foreground,
          ),
        ),
      ],
    );
  }
}

/// One section: its faculty, slot and school, marked when it is the student's.
class SectionTile extends StatelessWidget {
  const SectionTile({
    super.key,
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
      padding: const .symmetric(horizontal: Space.md + 2, vertical: Space.md),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: .start,
              children: [
                Row(
                  children: [
                    Flexible(
                      child: Text(
                        section.faculty,
                        maxLines: 1,
                        overflow: .ellipsis,
                        style: context.theme.typography.body.sm.copyWith(
                          fontWeight: .w700,
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
