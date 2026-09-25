import 'dart:developer' show log;

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/weightage_totals.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/more/domain/gpa_calculator.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/grade_badge.dart';
import 'package:vitapmate/features/more/presentation/providers/grades_provider.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesPage extends HookConsumerWidget {
  const GradesPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(gradesProvider);
    final semAsync = ref.watch(semesterIdProvider);
    final semData = semAsync.value;
    final stateValue = state.value;
    final stateSems = stateValue?.semesters ?? const <SemesterInfo>[];
    final semesters = stateSems.isNotEmpty
        ? stateSems
        : (semData?.semesters ?? const []);
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

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        try {
          await ref.read(gradesProvider.future);
          await ref.read(gradesProvider.notifier).refresh();
        } catch (e, st) {
          log('auto refresh failed: $e', stackTrace: st);
        }
      });
      return null;
    }, const []);

    final history = ref.watch(gradeHistoryProvider).value;
    final creditsByCode = {
      for (final r in history?.records ?? const <GradeHistoryRecord>[])
        r.courseCode.trim().toUpperCase(): double.tryParse(r.credits.trim()),
    };

    return RefreshIndicator(
      backgroundColor: context.theme.colors.background,
      color: context.theme.colors.foreground,
      onRefresh: refresh,
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(
          Space.sm,
          Space.sm,
          Space.sm,
          Space.lg,
        ),
        children: [
          if (semLoading)
            const Skeleton(height: 36, radius: Radii.pill)
          else if (semLoadError)
            FButton(
              variant: FButtonVariant.outline,
              prefix: const Icon(FLucideIcons.rotateCw),
              onPress: () => ref.invalidate(semesterIdProvider),
              child: const Text("Couldn't load semesters. Retry"),
            )
          else
            _SemesterChips(
              semesters: semesters,
              selectedId: selectedSemesterId,
              onSelect: (id) async {
                try {
                  await ref.read(gradesProvider.notifier).selectSemester(id);
                } catch (e) {
                  log("$e");
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
                final sorted = [...data.gradeView.courses]
                  ..sort(
                    (a, b) => (int.tryParse(a.serial) ?? 0).compareTo(
                      int.tryParse(b.serial) ?? 0,
                    ),
                  );
                return Column(
                  key: ValueKey('data_${data.selectedSemesterId}'),
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (sorted.isEmpty)
                      const EmptyState(
                        icon: FLucideIcons.school,
                        title: 'No grades for this semester',
                        message: 'Grades appear after results are declared.',
                      )
                    else ...[
                      _SemesterSummary(
                        courses: sorted,
                        creditsByCode: creditsByCode,
                      ),
                      const SizedBox(height: Space.md),
                      for (final (i, c) in sorted.indexed)
                        Padding(
                          padding: const EdgeInsets.only(bottom: Space.sm),
                          child: EnterFade(
                            index: i,
                            child: _GradeCard(course: c),
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
    );
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

  Future<void> _toggle() async {
    setState(() => _expanded = !_expanded);
    if (_expanded) {
      _controller.forward();
      final s = ref.read(gradesProvider).value;
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
          log("$e");
        }
      } else if (has && hasMarkerRange && !loading) {
        try {
          await ref
              .read(gradesProvider.notifier)
              .loadDetails(widget.course.courseId, force: true);
        } catch (e) {
          log("$e");
        }
      }
    } else {
      _controller.reverse();
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final state = ref.watch(gradesProvider).value;
    final detail = state?.detailsByCourseId[widget.course.courseId];
    final loading =
        state?.loadingDetailsFor.contains(widget.course.courseId) ?? false;
    final c = widget.course;

    return Surface(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          PressScale(
            scale: 0.99,
            semanticsLabel: '${c.courseTitle}, grade ${c.grade}',
            onPress: _toggle,
            child: Padding(
              padding: const EdgeInsets.all(Space.md + 2),
              child: Row(
                children: [
                  GradeBadge(grade: c.grade),
                  const SizedBox(width: Space.md),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          c.courseTitle,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: typography.body.md.copyWith(
                            height: 1.25,
                            fontWeight: FontWeight.w600,
                            color: colors.foreground,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${c.courseCode} · ${c.courseType}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: Space.md),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    children: [
                      Text(
                        c.grandTotal,
                        style: typography.body.lg.copyWith(
                          fontWeight: FontWeight.w500,
                          color: colors.foreground,
                          fontFeatures: const [FontFeature.tabularFigures()],
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
                    turns: _expanded ? 0.5 : 0,
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
            sizeFactor: _animation,
            alignment: Alignment.topCenter,
            child: Padding(
              padding: EdgeInsets.zero,
              child: _GradeDetailsPanel(
                detail: detail,
                loading: loading,
                gradingType: c.gradingType,
                grade: c.grade,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _GradeDetailsPanel extends StatelessWidget {
  final GradeDetailsData? detail;
  final bool loading;
  final String gradingType;
  final String grade;
  const _GradeDetailsPanel({
    required this.detail,
    required this.loading,
    required this.gradingType,
    required this.grade,
  });

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    if (detail == null) {
      return Padding(
        padding: const EdgeInsets.all(Space.md),
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
            : Text(
                "Couldn't load the mark breakdown. Collapse and try again.",
                style: typography.body.sm.copyWith(
                  color: colors.mutedForeground,
                ),
              ),
      );
    }

    final ranges = [
      for (final r in detail!.gradeRanges)
        if (r.range.replaceAll('#', '').trim().isNotEmpty)
          (r.grade.trim().toUpperCase(), _shortRange(r.range)),
    ];
    final sections = _groupMarksBySection(detail!);

    return Padding(
      padding: const EdgeInsets.fromLTRB(
        Space.md + 2,
        Space.xs,
        Space.md + 2,
        Space.md,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (ranges.isNotEmpty) ...[
            _Label(
              gradingType.trim().isEmpty
                  ? 'GRADE CUT-OFFS'
                  : 'GRADE CUT-OFFS · ${gradingType.trim()}',
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
                    achieved: g == grade.trim().toUpperCase(),
                  ),
              ],
            ),
          ],
          for (final section in sections) ...[
            const SizedBox(height: Space.lg),
            Row(
              children: [
                Expanded(child: _Label(section.sectionTitle.toUpperCase())),
                if (section.total.isNotEmpty)
                  Text(
                    '${section.total} weighted',
                    style: typography.body.xs.copyWith(
                      fontWeight: FontWeight.w600,
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
        ],
      ),
    );
  }

  /// ">=83 and <88" → "83–88", ">=88" → "≥88", "<50" → "<50".
  static String _shortRange(String raw) {
    final r = raw.replaceAll('#', '').trim();
    final nums = RegExp(
      r'\d+(?:\.\d+)?',
    ).allMatches(r).map((m) => m[0]!).toList();
    if (nums.length >= 2) return '${nums[0]}–${nums[1]}';
    if (nums.length == 1) {
      if (r.startsWith('<')) return '<${nums[0]}';
      return '≥${nums[0]}';
    }
    return r;
  }

  List<_MarkSection> _groupMarksBySection(GradeDetailsData detail) {
    final classTypes = detail.classCourseType
        .split('|')
        .map((e) => e.trim())
        .where((e) => e.isNotEmpty)
        .toList();
    final classNumbers = detail.classNumber
        .split('|')
        .map((e) => e.trim())
        .where((e) => e.isNotEmpty)
        .toList();
    final classNumberByType = <String, String>{};
    for (int i = 0; i < classTypes.length; i++) {
      classNumberByType[classTypes[i]] = i < classNumbers.length
          ? classNumbers[i]
          : "";
    }

    final grouped = <String, List<GradeDetailMark>>{};
    final order = <String>[];
    for (final mark in detail.marks) {
      String section = detail.classCourseType;
      String title = mark.markTitle;
      if (mark.markTitle.contains("•")) {
        final parts = mark.markTitle.split("•");
        if (parts.length >= 2) {
          section = parts.first.trim();
          title = parts.sublist(1).join("•").trim();
        }
      }
      if (!grouped.containsKey(section)) {
        order.add(section);
      }
      grouped.putIfAbsent(section, () => []);
      grouped[section]!.add(mark.copyWith(markTitle: title));
    }

    return [
      for (final section in order)
        _MarkSection(
          sectionTitle: section,
          classNumber: classNumberByType[section] ?? "",
          total: _sumWeightageMarks(grouped[section] ?? const []),
          marks: grouped[section] ?? const [],
        ),
    ];
  }

  String _sumWeightageMarks(List<GradeDetailMark> marks) {
    final totals = calculateWeightageTotals<GradeDetailMark>(
      marks,
      titleOf: (m) => m.markTitle,
      gainedOf: (m) => m.weightageMark,
      possibleOf: (m) => m.weightage,
    );
    String fmt(double v) =>
        v == v.truncateToDouble() ? v.toStringAsFixed(0) : v.toStringAsFixed(1);
    if (totals.gained == 0) return "";
    return totals.possible > 0
        ? '${fmt(totals.gained)} / ${fmt(totals.possible)}'
        : fmt(totals.gained);
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
      fontWeight: FontWeight.w600,
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
      padding: const EdgeInsets.symmetric(horizontal: Space.sm, vertical: 4),
      decoration: BoxDecoration(
        color: achieved ? tone.subtle : const Color(0x00000000),
        borderRadius: BorderRadius.circular(Radii.sm),
        border: Border.all(color: achieved ? tone.base : colors.border),
      ),
      child: Text.rich(
        TextSpan(
          children: [
            TextSpan(
              text: '$grade ',
              style: TextStyle(
                fontWeight: FontWeight.w700,
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
        style: context.theme.typography.body.xs.copyWith(
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
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
      padding: const EdgeInsets.symmetric(vertical: Space.sm),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  mark.markTitle,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w500,
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
                style: typography.body.sm.copyWith(
                  fontWeight: FontWeight.w500,
                  color: colors.foreground,
                  fontFeatures: const [FontFeature.tabularFigures()],
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
                style: typography.body.xs.copyWith(
                  color: colors.mutedForeground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _MarkSection {
  final String sectionTitle;
  final String classNumber;
  final String total;
  final List<GradeDetailMark> marks;

  const _MarkSection({
    required this.sectionTitle,
    required this.classNumber,
    required this.total,
    required this.marks,
  });
}

class _SemesterChips extends StatelessWidget {
  const _SemesterChips({
    required this.semesters,
    required this.selectedId,
    required this.onSelect,
  });

  final List<SemesterInfo> semesters;
  final String selectedId;
  final ValueChanged<String> onSelect;

  /// "Winter Semester 2025-26 Freshers" → "Winter 25-26 Freshers".
  static String short(String name) => name
      .replaceAll(RegExp(r'\s+'), ' ')
      .replaceAllMapped(
        RegExp(r'\s*Semester\s+20(\d{2})-(\d{2})'),
        (m) => ' ${m.group(1)}-${m.group(2)}',
      )
      .trim();

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          for (final sem in semesters)
            Padding(
              padding: const EdgeInsets.only(right: Space.sm),
              child: PressScale(
                scale: 0.95,
                semanticsLabel: sem.name,
                onPress: () => onSelect(sem.id),
                child: AnimatedContainer(
                  duration: Motion.medium,
                  padding: const EdgeInsets.symmetric(
                    horizontal: Space.md,
                    vertical: Space.sm - 1,
                  ),
                  decoration: BoxDecoration(
                    color: sem.id == selectedId ? colors.primary : colors.card,
                    borderRadius: BorderRadius.circular(Radii.pill),
                    border: Border.all(
                      color: sem.id == selectedId
                          ? colors.primary
                          : colors.border,
                    ),
                  ),
                  child: Text(
                    short(sem.name),
                    style: context.theme.typography.body.sm.copyWith(
                      fontWeight: FontWeight.w500,
                      color: sem.id == selectedId
                          ? colors.primaryForeground
                          : colors.foreground,
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Semester GPA from the grades, weighted by credits looked up in grade
/// history. Courses without credits or a points grade (e.g. P) are left out.
class _SemesterSummary extends StatelessWidget {
  const _SemesterSummary({required this.courses, required this.creditsByCode});

  final List<GradeCourseRecord> courses;
  final Map<String, double?> creditsByCode;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    var credits = 0.0;
    var points = 0.0;
    var counted = 0;
    for (final c in courses) {
      final grade = Grade.tryParse(c.grade);
      final cr = creditsByCode[c.courseCode.trim().toUpperCase()];
      if (grade == null || cr == null || cr <= 0) continue;
      credits += cr;
      points += cr * grade.points;
      counted++;
    }
    final gpa = credits > 0 ? points / credits : null;
    final counts = <String, int>{};
    for (final c in courses) {
      final g = c.grade.trim().toUpperCase();
      if (g.isEmpty || g == '-') continue;
      counts[g] = (counts[g] ?? 0) + 1;
    }
    const order = ['S', 'A', 'B', 'C', 'D', 'E', 'F', 'N', 'P'];
    int rank(String g) => order.contains(g) ? order.indexOf(g) : order.length;
    final sortedCounts = counts.entries.toList()
      ..sort((a, b) => rank(a.key).compareTo(rank(b.key)));

    return Surface(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'SEMESTER GPA',
            style: typography.body.xs.copyWith(
              fontWeight: FontWeight.w600,
              letterSpacing: 0.6,
              color: colors.mutedForeground,
            ),
          ),
          gpa == null
              ? Text(
                  '—',
                  style: typography.display.xl2.copyWith(
                    color: colors.mutedForeground,
                  ),
                )
              : CountUp(
                  value: gpa,
                  decimals: 2,
                  style: typography.display.xl2.copyWith(
                    height: 1.1,
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
          Text(
            gpa == null
                ? 'Needs credits from grade history'
                : '${courses.length} courses · ${_num(credits)} credits',
            style: typography.body.xs.copyWith(color: colors.mutedForeground),
          ),
          if (gpa != null && counted < courses.length)
            Text(
              '${courses.length - counted} without grade points not counted',
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
          if (counts.isNotEmpty) ...[
            const SizedBox(height: Space.md),
            Wrap(
              spacing: Space.xs + 2,
              runSpacing: Space.xs,
              children: [
                for (final e in sortedCounts)
                  ToneBadge(
                    label: '${e.key} ${e.value}',
                    tone: gradeTone(context, e.key),
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }

  static String _num(double v) =>
      v == v.roundToDouble() ? v.toStringAsFixed(0) : v.toStringAsFixed(1);
}
