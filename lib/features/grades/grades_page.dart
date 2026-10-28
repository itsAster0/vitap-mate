import 'dart:math' as math;

import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/features/marks/marks.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/gpa/gpa.dart';
import 'package:vitapmate/features/grade_history/grade_history.dart';
import 'package:vitapmate/ui/grade_badge.dart';
import 'package:vitapmate/features/grades/grades.dart';
import 'package:vitapmate/vtop/semesters.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';

/// "4" for whole numbers, else one decimal: "2.5".
String _fmt(double v) =>
    v == v.roundToDouble() ? v.toStringAsFixed(0) : v.toStringAsFixed(1);

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
              _SemesterChips(
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
                        _SemesterSummary(
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
                                child: _GradeCard(course: c),
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

class _GradeCard extends HookConsumerWidget {
  const _GradeCard({required this.course});

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
            child: _GradeDetailsPanel(
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

class _GradeDetailsPanel extends StatelessWidget {
  const _GradeDetailsPanel({
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
          (r.grade.trim().toUpperCase(), _shortRange(r.range)),
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
          for (final section in _sections(detail)) ...[
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

  /// ">=83 and <88" → "83–88", ">=88" → "≥88", "<50" → "<50".
  static String _shortRange(String raw) {
    final r = raw.replaceAll('#', '').trim();
    final nums = [
      for (final m in RegExp(r'\d+(?:\.\d+)?').allMatches(r)) m[0]!,
    ];
    return switch (nums) {
      [final lo, final hi, ...] => '$lo–$hi',
      [final n] => r.startsWith('<') ? '<$n' : '≥$n',
      _ => r,
    };
  }

  /// Marks grouped by the section before "•" in their title ("Theory •
  /// CAT-1"), in first-seen order, each with its weighted total.
  static List<({String title, String total, List<GradeDetailMark> marks})>
  _sections(GradeDetailsData detail) {
    final grouped = <String, List<GradeDetailMark>>{};
    for (final mark in detail.marks) {
      final parts = mark.markTitle.split("•");
      final (section, title) = parts.length >= 2
          ? (parts.first.trim(), parts.skip(1).join("•").trim())
          : (detail.classCourseType, mark.markTitle);
      grouped
          .putIfAbsent(section, () => [])
          .add(mark.copyWith(markTitle: title));
    }
    return [
      for (final MapEntry(key: title, value: marks) in grouped.entries)
        (title: title, total: _weightedTotal(marks), marks: marks),
    ];
  }

  static String _weightedTotal(List<GradeDetailMark> marks) {
    final totals = calculateWeightageTotals<GradeDetailMark>(
      marks,
      titleOf: (m) => m.markTitle,
      gainedOf: (m) => m.weightageMark,
      possibleOf: (m) => m.weightage,
    );
    if (totals.gained == 0) return "";
    return totals.possible > 0
        ? '${_fmt(totals.gained)} / ${_fmt(totals.possible)}'
        : _fmt(totals.gained);
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
      scrollDirection: .horizontal,
      child: Row(
        children: [
          for (final sem in semesters)
            Padding(
              padding: const .only(right: Space.sm),
              child: PressScale(
                scale: 0.95,
                semanticsLabel: sem.name,
                onPress: () => onSelect(sem.id),
                child: AnimatedContainer(
                  duration: Motion.medium,
                  padding: const .symmetric(
                    horizontal: Space.md,
                    vertical: Space.sm - 1,
                  ),
                  decoration: BoxDecoration(
                    color: sem.id == selectedId ? colors.primary : colors.card,
                    borderRadius: .circular(Radii.pill),
                    border: .all(
                      color: sem.id == selectedId
                          ? colors.primary
                          : colors.border,
                    ),
                  ),
                  child: Text(
                    short(sem.name),
                    style: context.theme.typography.body.sm.copyWith(
                      fontWeight: .w500,
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
  const _SemesterSummary({
    required this.courses,
    required this.creditsByCode,
    required this.onCourse,
  });

  final List<GradeCourseRecord> courses;
  final Map<String, double?> creditsByCode;
  final ValueChanged<GradeCourseRecord> onCourse;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    var credits = 0.0;
    var points = 0.0;
    final counts = <String, int>{};
    for (final c in courses) {
      final grade = Grade.tryParse(c.grade);
      final cr = creditsByCode[c.courseCode.trim().toUpperCase()];
      if (grade != null && cr != null && cr > 0) {
        credits += cr;
        points += cr * grade.points;
      }
      final g = c.grade.trim().toUpperCase();
      if (g.isNotEmpty && g != '-') counts[g] = (counts[g] ?? 0) + 1;
    }
    final gpa = credits > 0 ? points / credits : null;
    const order = ['S', 'A', 'B', 'C', 'D', 'E', 'F', 'N', 'P'];
    int rank(String g) => order.contains(g) ? order.indexOf(g) : order.length;
    final sortedCounts = counts.entries.toList()
      ..sort((a, b) => rank(a.key).compareTo(rank(b.key)));
    final label = typography.body.xs.copyWith(
      fontWeight: .w600,
      letterSpacing: 0.6,
      color: colors.mutedForeground,
    );

    return Surface(
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          Row(
            children: [
              Expanded(child: Text('SEMESTER GPA', style: label)),
              if (gpa != null) Text('${_fmt(credits)} credits', style: label),
            ],
          ),
          Row(
            crossAxisAlignment: .end,
            children: [
              Expanded(
                child: gpa == null
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
                          fontWeight: .w600,
                          color: colors.foreground,
                        ),
                      ),
              ),
              Padding(
                padding: const .only(bottom: 4),
                child: Text(
                  gpa == null
                      ? 'Needs credits from grade history'
                      : '${courses.length} courses',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.md),
          _CourseBar(
            courses: courses,
            creditsByCode: creditsByCode,
            onCourse: onCourse,
          ),
          if (counts.isNotEmpty) ...[
            const SizedBox(height: Space.md),
            Wrap(
              spacing: Space.md,
              runSpacing: Space.xs,
              children: [
                for (final MapEntry(key: grade, value: count) in sortedCounts)
                  Row(
                    mainAxisSize: .min,
                    children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: gradeTone(context, grade).base,
                          borderRadius: .circular(2),
                        ),
                      ),
                      const SizedBox(width: Space.xs),
                      Text(
                        '$grade $count',
                        style: typography.body.xs.nums(
                          color: colors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

/// One bar for the semester: a segment per course, as wide as its credits and
/// coloured by its grade. Tap a segment to jump to that course.
class _CourseBar extends StatelessWidget {
  const _CourseBar({
    required this.courses,
    required this.creditsByCode,
    required this.onCourse,
  });

  final List<GradeCourseRecord> courses;
  final Map<String, double?> creditsByCode;
  final ValueChanged<GradeCourseRecord> onCourse;

  @override
  Widget build(BuildContext context) => TweenAnimationBuilder<double>(
    tween: Tween(begin: 0, end: 1),
    duration: Motion.slow * 2,
    curve: Curves.easeOutCubic,
    builder: (context, t, _) => Row(
      children: [
        for (final (i, c) in courses.indexed) ...[
          if (i > 0) const SizedBox(width: 3),
          Expanded(
            // Non-credit courses still get a sliver so they're visible.
            flex: math.max(
              5,
              ((creditsByCode[c.courseCode.trim().toUpperCase()] ?? 0) * 10)
                  .round(),
            ),
            child: PressScale(
              scale: 0.9,
              semanticsLabel: '${c.courseTitle}, grade ${c.grade}',
              onPress: () => onCourse(c),
              child: Padding(
                // Taller hit area than the bar itself.
                padding: const .symmetric(vertical: 6),
                child: Opacity(
                  opacity: t,
                  child: Container(
                    height: 10,
                    decoration: BoxDecoration(
                      color: gradeTone(context, c.grade.trim()).base,
                      borderRadius: .circular(3),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ],
    ),
  );
}
