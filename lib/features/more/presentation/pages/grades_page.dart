import 'dart:developer' show log;

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/providers/grades_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesPage extends HookConsumerWidget {
  const GradesPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(gradesProvider);
    final semAsync = ref.watch(semesterIdProvider);
    final semData = semAsync.valueOrNull;
    final stateValue = state.valueOrNull;
    final stateSems = stateValue?.semesters ?? const <SemesterInfo>[];
    final semesters =
        stateSems.isNotEmpty ? stateSems : (semData?.semesters ?? const []);
    final semLoading = semesters.isEmpty && semAsync.isLoading;
    final semLoadError = semesters.isEmpty && semAsync.hasError;
    final selectedSemesterId =
        stateValue?.selectedSemesterId ??
        (semesters.isNotEmpty ? semesters.first.id : "");

    Future<void> refresh() async {
      try {
        await ref.read(semesterIdProvider.future);
        await ref.read(gradesProvider.notifier).refresh();
      } catch (e) {
        log("$e");

        if (context.mounted) disCommonToast(context, e);
      }
    }

    return Container(
      color: context.theme.colors.background,
      child: RefreshIndicator(
        backgroundColor: context.theme.colors.primary,
        color: context.theme.colors.primaryForeground,
        onRefresh: refresh,
        displacement: 72,
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(8, 8, 8, 20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              FTileGroup(
                children: [
                  if (semLoading)
                    FTile(
                      prefix: const Icon(FIcons.calendarDays),
                      title: const Text("Semester"),
                      subtitle: const Text("Loading semesters..."),
                      suffix: const Icon(FIcons.loaderCircle),
                    )
                  else if (semLoadError)
                    FTile(
                      prefix: const Icon(FIcons.calendarDays),
                      title: const Text("Semester"),
                      subtitle: const Text(
                        "Unable to load semesters. Tap to retry",
                      ),
                      suffix: const Icon(FIcons.rotateCw),
                      onPress: () {
                        ref.invalidate(semesterIdProvider);
                      },
                    )
                  else
                    FSelectMenuTile<String>(
                      key: ValueKey(
                        "grade_sem_${selectedSemesterId}_${semesters.length}",
                      ),
                      prefix: const Icon(FIcons.calendarDays),
                      title: const Text("Semester"),
                      details: Text(
                        _semesterNameFromList(semesters, selectedSemesterId),
                      ),
                      initialValue:
                          selectedSemesterId.isEmpty
                              ? null
                              : selectedSemesterId,
                      menu: [
                        for (final sem in semesters)
                          FSelectTile<String>(
                            title: Text(sem.name),
                            value: sem.id,
                          ),
                      ],
                      onChange: (value) async {
                        final selected = value.firstOrNull;
                        if (selected == null) return;
                        try {
                          await ref
                              .read(gradesProvider.notifier)
                              .selectSemester(selected);
                        } catch (e) {
                          if (context.mounted) disCommonToast(context, e);
                        }
                      },
                    ),
                ],
              ),
              const SizedBox(height: 10),
              state.when(
                loading:
                    () => SizedBox(
                      height: MediaQuery.of(context).size.height * 0.7,
                      child: const _CenterInfo(
                        title: "Loading grades...",
                        icon: FIcons.loaderCircle,
                      ),
                    ),
                error:
                    (e, _) => SizedBox(
                      height: MediaQuery.of(context).size.height * 0.7,
                      child: _CenterInfo(
                        title: "Unable to load grades",
                        subtitle: commonErrorMessage(e),
                        icon: FIcons.triangleAlert,
                      ),
                    ),
                data: (data) {
                  final sorted = [...data.gradeView.courses]..sort(
                    (a, b) => (int.tryParse(a.serial) ?? 0).compareTo(
                      int.tryParse(b.serial) ?? 0,
                    ),
                  );

                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (sorted.isEmpty)
                        SizedBox(
                          height: MediaQuery.of(context).size.height * 0.4,
                          child: const _CenterInfo(
                            title: "No grades found",
                            subtitle: "Try a different semester.",
                            icon: FIcons.school,
                          ),
                        )
                      else
                        ...sorted.map((c) => _GradeCard(course: c)),
                      if (data.gradeView.updateTime > BigInt.zero)
                        Center(
                          child: Padding(
                            padding: const EdgeInsets.only(top: 12),
                            child: Text(
                              "Data updated on ${formatUnixTimestamp(data.gradeView.updateTime.toInt())}",
                              style: TextStyle(
                                fontSize: 13,
                                color: context.theme.colors.mutedForeground,
                                fontStyle: FontStyle.italic,
                              ),
                            ),
                          ),
                        ),
                    ],
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }

  String _semesterNameFromList(
    List<SemesterInfo> sems,
    String selectedSemesterId,
  ) {
    final selected = sems.where((e) => e.id == selectedSemesterId);
    if (selected.isNotEmpty) return selected.first.name;
    return "Choose Semester";
  }
}

class _GradeCard extends ConsumerStatefulWidget {
  final GradeCourseRecord course;
  const _GradeCard({required this.course});

  @override
  ConsumerState<_GradeCard> createState() => _GradeCardState();
}

class _GradeCardState extends ConsumerState<_GradeCard>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _animation;
  bool _expanded = false;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 250),
    );
    _animation = CurvedAnimation(parent: _controller, curve: Curves.easeInOut);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Color _gradeColor(String grade) {
    switch (grade.trim().toUpperCase()) {
      case 'S':
      case 'A':
        return MarksColors.excellentColor;
      case 'B':
      case 'C':
        return MarksColors.averageColor;
      case 'F':
        return MarksColors.failedText;
      default:
        return MarksColors.secondaryText;
    }
  }

  Future<void> _toggle() async {
    setState(() => _expanded = !_expanded);
    if (_expanded) {
      _controller.forward();
      final s = ref.read(gradesProvider).valueOrNull;
      final has =
          s?.detailsByCourseId.containsKey(widget.course.courseId) ?? false;
      final loading =
          s?.loadingDetailsFor.contains(widget.course.courseId) ?? false;
      final hasMarkerRange =
          s?.detailsByCourseId[widget.course.courseId]?.gradeRanges.any(
            (r) => r.range.contains('#'),
          ) ??
          false;
      if (!has && !loading) {
        try {
          await ref
              .read(gradesProvider.notifier)
              .loadDetails(widget.course.courseId);
        } catch (e) {
          if (mounted) disCommonToast(context, e);
        }
      } else if (has && hasMarkerRange && !loading) {
        try {
          await ref
              .read(gradesProvider.notifier)
              .loadDetails(widget.course.courseId, force: true);
        } catch (e) {
          if (mounted) disCommonToast(context, e);
        }
      }
    } else {
      _controller.reverse();
    }
  }

  @override
  Widget build(BuildContext context) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final state = ref.watch(gradesProvider).valueOrNull;
    final detail = state?.detailsByCourseId[widget.course.courseId];
    final loading =
        state?.loadingDetailsFor.contains(widget.course.courseId) ?? false;

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        children: [
          FTappable(
            onPress: _toggle,
            child: Container(
              decoration: BoxDecoration(
                gradient:
                    !darkMode
                        ? const LinearGradient(
                          colors: [
                            MarksColors.theoryCardBackground,
                            MarksColors.theoryCardSecondary,
                          ],
                          begin: Alignment.topLeft,
                          end: Alignment.bottomRight,
                        )
                        : null,
                borderRadius: BorderRadius.circular(12),
                color: darkMode ? context.theme.colors.primaryForeground : null,
                boxShadow: const [
                  BoxShadow(
                    color: MarksColors.cardShadow,
                    blurRadius: 10,
                    offset: Offset(0, 4),
                  ),
                ],
                border: Border.all(
                  color: context.theme.colors.primaryForeground.withValues(
                    alpha: .8,
                  ),
                  width: 1,
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.all(14),
                child: Column(
                  children: [
                    Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: context.theme.colors.primaryForeground
                                .withValues(alpha: 0.9),
                            borderRadius: BorderRadius.circular(10),
                          ),
                          child: const Icon(
                            FIcons.graduationCap,
                            color: MarksColors.theoryIcon,
                            size: 20,
                          ),
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                widget.course.courseTitle,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  fontWeight: FontWeight.w700,
                                  fontSize: 16,
                                  color:
                                      darkMode
                                          ? context.theme.colors.primary
                                          : MarksColors.primaryText,
                                ),
                              ),
                              const SizedBox(height: 2),
                              Text(
                                "${widget.course.courseCode} • ${widget.course.courseType}",
                                style: TextStyle(
                                  fontSize: 12,
                                  color:
                                      darkMode
                                          ? context.theme.colors.primary
                                          : MarksColors.secondaryText,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 10,
                            vertical: 7,
                          ),
                          decoration: BoxDecoration(
                            color: context.theme.colors.primaryForeground
                                .withValues(alpha: 0.9),
                            borderRadius: BorderRadius.circular(20),
                            border: Border.all(
                              color: _gradeColor(
                                widget.course.grade,
                              ).withValues(alpha: 0.4),
                            ),
                          ),
                          child: Text(
                            widget.course.grade,
                            style: TextStyle(
                              fontWeight: FontWeight.w700,
                              color: _gradeColor(widget.course.grade),
                            ),
                          ),
                        ),
                        const SizedBox(width: 8),
                        AnimatedRotation(
                          turns: _expanded ? 0.5 : 0,
                          duration: const Duration(milliseconds: 250),
                          child: const Icon(Icons.keyboard_arrow_down),
                        ),
                      ],
                    ),
                    const SizedBox(height: 10),
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            "Grand Total: ${widget.course.grandTotal}",
                            style: TextStyle(
                              color:
                                  darkMode
                                      ? context.theme.colors.primary
                                      : MarksColors.primaryText,
                              fontWeight: FontWeight.w700,
                              fontSize: 12,
                            ),
                          ),
                        ),
                        _CardReloadAction(
                          loading: loading,
                          onPress: () async {
                            try {
                              await ref
                                  .read(gradesProvider.notifier)
                                  .loadDetails(
                                    widget.course.courseId,
                                    force: true,
                                  );
                            } catch (e) {
                              if (!mounted) return;
                              disCommonToast(this.context, e);
                            }
                          },
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),
          AnimatedBuilder(
            animation: _animation,
            builder:
                (context, child) =>
                    FCollapsible(value: _animation.value, child: child!),
            child: _GradeDetailsPanel(
              detail: detail,
              loading: loading,
              gradingType: widget.course.gradingType,
            ),
          ),
        ],
      ),
    );
  }
}

class _CardReloadAction extends StatelessWidget {
  final bool loading;
  final Future<void> Function() onPress;

  const _CardReloadAction({required this.loading, required this.onPress});

  @override
  Widget build(BuildContext context) {
    return FTappable(
      onPress: loading ? null : onPress,
      child: Container(
        padding: const EdgeInsets.all(7),
        decoration: BoxDecoration(
          color: context.theme.colors.primaryForeground.withValues(alpha: 0.9),
          borderRadius: BorderRadius.circular(999),
          border: Border.all(color: context.theme.colors.border),
        ),
        child: Icon(
          loading ? FIcons.loaderCircle : FIcons.rotateCw,
          size: 14,
          color: context.theme.colors.primary,
        ),
      ),
    );
  }
}

class _GradeDetailsPanel extends StatelessWidget {
  final GradeDetailsData? detail;
  final bool loading;
  final String gradingType;
  const _GradeDetailsPanel({
    required this.detail,
    required this.loading,
    required this.gradingType,
  });

  @override
  Widget build(BuildContext context) {
    if (detail == null && loading) {
      return Container(
        padding: const EdgeInsets.all(12),
        child: Center(
          child: Icon(
            FIcons.loaderCircle,
            size: 22,
            color: context.theme.colors.mutedForeground,
          ),
        ),
      );
    }
    if (detail == null) {
      return Container(
        padding: const EdgeInsets.all(12),
        child: Text(
          "Tap card to load detailed marks",
          style: TextStyle(color: context.theme.colors.mutedForeground),
        ),
      );
    }

    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 4),
      decoration: BoxDecoration(
        color: context.theme.colors.primaryForeground,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: context.theme.colors.border),
      ),
      padding: const EdgeInsets.all(10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            decoration: BoxDecoration(
              color: context.theme.colors.background,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: context.theme.colors.border),
            ),
            child: Wrap(
              spacing: 10,
              runSpacing: 6,
              children: [
                Text(
                  "Grading Type: $gradingType",
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                    color: context.theme.colors.primary,
                  ),
                ),
                Text(
                  _formatGradeRanges(detail!.gradeRanges),
                  style: TextStyle(
                    fontSize: 12,
                    color: context.theme.colors.mutedForeground,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          Text(
            "Class: ${detail!.classNumber} • ${detail!.classCourseType}",
            style: TextStyle(
              fontSize: 13,
              color: context.theme.colors.mutedForeground,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: 8),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  decoration: BoxDecoration(
                    color: context.theme.colors.primaryForeground.withValues(
                      alpha: 0.5,
                    ),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 6,
                  ),
                  child: _row(
                    context,
                    "Serial Number",
                    "Mark Title",
                    "Scored Mark",
                    "Maximum Mark",
                    "Weightage (%)",
                    "Weightage Mark",
                    true,
                  ),
                ),
                const SizedBox(height: 8),
                for (int i = 0; i < detail!.marks.length; i++) ...[
                  _row(
                    context,
                    detail!.marks[i].serial,
                    detail!.marks[i].markTitle,
                    detail!.marks[i].scoredMark,
                    detail!.marks[i].maxMark,
                    detail!.marks[i].weightage,
                    detail!.marks[i].weightageMark,
                    false,
                  ),
                  if (i != detail!.marks.length - 1)
                    Container(
                      margin: const EdgeInsets.only(bottom: 6),
                      height: 1,
                      width: 760,
                      color: context.theme.colors.border.withValues(alpha: 0.7),
                    ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _row(
    BuildContext context,
    String a,
    String b,
    String c,
    String d,
    String e,
    String f,
    bool header,
  ) {
    final style = TextStyle(
      fontSize: header ? 13 : 12,
      fontWeight: header ? FontWeight.w700 : FontWeight.w400,
      color:
          header
              ? context.theme.colors.primary
              : context.theme.colors.foreground,
    );
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Row(
        children: [
          _cell(a, 110, style),
          _cell(b, 220, style),
          _cell(c, 110, style),
          _cell(d, 120, style),
          _cell(e, 120, style),
          _cell(f, 120, style),
        ],
      ),
    );
  }

  Widget _cell(String txt, double width, TextStyle style) => SizedBox(
    width: width,
    child: Text(
      txt,
      maxLines: 2,
      overflow: TextOverflow.ellipsis,
      style: style.copyWith(height: 1.25),
    ),
  );

  String _formatGradeRanges(List<GradeRange> ranges) {
    if (ranges.isEmpty) return "Grade ranges unavailable";
    final normalized =
        ranges
            .map(
              (r) => GradeRange(
                grade: r.grade,
                range: r.range.replaceAll('#', '').trim(),
              ),
            )
            .where((r) => r.range.isNotEmpty)
            .toList();
    if (normalized.isEmpty) return "Grade ranges unavailable";
    return normalized.map((r) => "${r.grade}: ${r.range}").join("   ");
  }
}

class _CenterInfo extends StatelessWidget {
  final String title;
  final String? subtitle;
  final IconData icon;
  const _CenterInfo({required this.title, this.subtitle, required this.icon});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 36, color: context.theme.colors.mutedForeground),
          const SizedBox(height: 10),
          Text(
            title,
            style: TextStyle(
              color: context.theme.colors.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
          if (subtitle != null) ...[
            const SizedBox(height: 6),
            Text(
              subtitle!,
              textAlign: TextAlign.center,
              style: TextStyle(color: context.theme.colors.mutedForeground),
            ),
          ],
        ],
      ),
    );
  }
}
