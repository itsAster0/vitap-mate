import 'dart:math' as math;

import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/gpa/gpa.dart';
import 'package:vitapmate/features/grade_history/grade_history.dart';
import 'package:vitapmate/ui/grade_badge.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';

const _gradeOrder = ['S', 'A', 'B', 'C', 'D', 'E', 'F', 'N'];

/// Courses grouped by exam session ("Jan-2025"), in the order VTOP lists them.
Map<String, List<GradeHistoryRecord>> _bySession(
  Iterable<GradeHistoryRecord> records,
) {
  final groups = <String, List<GradeHistoryRecord>>{};
  for (final r in records) {
    groups.putIfAbsent(r.examMonth.trim(), () => []).add(r);
  }
  return groups;
}

/// Credit-weighted GPA of [records]; pass/fail and N grades carry no points
/// and are left out. Null when nothing counts.
double? _sessionGpa(Iterable<GradeHistoryRecord> records) {
  var points = 0.0;
  var credits = 0.0;
  for (final r in records) {
    final grade = Grade.tryParse(r.grade);
    final c = Credits.tryParse(r.credits);
    if (grade == null || grade == Grade.n || c == null) continue;
    points += grade.points * c.value;
    credits += c.value;
  }
  return credits == 0 ? null : points / credits;
}

/// "Jan-2025" → "Jan '25", for chart labels.
String _shortSession(String month) => switch (month.trim().split('-')) {
  [final m, final year] when year.length == 4 => "$m '${year.substring(2)}",
  _ => month,
};

/// "Jan-2025" → "Jan 2025".
String _sessionLabel(String month) => month.replaceAll('-', ' ').trim();

class GradeHistoryPage extends HookConsumerWidget {
  const GradeHistoryPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data = ref.watch(gradeHistoryProvider);

    Future<void> reload() async {
      try {
        await ref.read(gradeHistoryProvider.notifier).refresh();
      } catch (e) {
        Log.info('features.grade_history', "$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(gradeHistoryProvider.notifier).refresh().catchError((e, st) {
          Log.info('features.grade_history', 'auto refresh failed: $e');
        });
      });
      return null;
    }, const []);

    return AnimatedSwitcher(
      duration: Motion.medium,
      child: data.when(
        skipLoadingOnRefresh: true,
        skipLoadingOnReload: true,
        loading: () => const Padding(
          key: ValueKey('loading'),
          padding: .all(Space.sm),
          child: Column(
            children: [
              Skeleton(height: 150, radius: Radii.lg),
              SizedBox(height: Space.lg),
              SkeletonList(count: 5, height: 80),
            ],
          ),
        ),
        error: (e, _) => EmptyState(
          key: const ValueKey('error'),
          icon: FLucideIcons.cloudAlert,
          title: "Couldn't load grade history",
          message: commonErrorMessage(e),
          action: FButton(
            variant: .outline,
            mainAxisSize: .min,
            onPress: reload,
            child: const Text('Try again'),
          ),
        ),
        data: (history) => ScreenRefresh(
          onRefresh: reload,
          tasks: const ['vtop_grade_history'],
          child: RefreshIndicator(
            key: const ValueKey('data'),
            onRefresh: reload,
            backgroundColor: context.theme.colors.background,
            color: context.theme.colors.foreground,
            child: _HistoryView(history: history),
          ),
        ),
      ),
    );
  }
}

class _HistoryView extends HookWidget {
  const _HistoryView({required this.history});

  final GradeHistoryData history;

  @override
  Widget build(BuildContext context) {
    final filter = useState<String?>(null);
    final present = {
      for (final r in history.records) r.grade.trim().toUpperCase(),
    };
    final grades = [
      for (final g in _gradeOrder)
        if (present.contains(g)) g,
      for (final g in present)
        if (!_gradeOrder.contains(g) && g != '-' && g.isNotEmpty) g,
    ];
    final shown = filter.value == null
        ? history.records
        : history.records
              .where((r) => r.grade.trim().toUpperCase() == filter.value)
              .toList();

    return ListView(
      padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.lg),
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        _CgpaHero(history: history),
        if (history.records.isNotEmpty) ...[
          SectionHeader(
            title: 'Courses',
            trailing: Text('${shown.length} of ${history.records.length}'),
          ),
          SingleChildScrollView(
            scrollDirection: .horizontal,
            child: Row(
              children: [
                _FilterChip(
                  label: 'All',
                  selected: filter.value == null,
                  onPress: () => filter.value = null,
                ),
                for (final g in grades)
                  _FilterChip(
                    label: g,
                    count: history.records
                        .where((r) => r.grade.trim().toUpperCase() == g)
                        .length,
                    tone: gradeTone(context, g),
                    selected: filter.value == g,
                    onPress: () => filter.value = filter.value == g ? null : g,
                  ),
              ],
            ),
          ),
          const SizedBox(height: Space.md),
        ],
        AnimatedSwitcher(
          duration: Motion.medium,
          switchInCurve: const Interval(0.3, 1, curve: Curves.easeOut),
          switchOutCurve: const Interval(0.7, 1, curve: Curves.easeIn),
          layoutBuilder: (current, previous) =>
              Stack(alignment: .topCenter, children: [...previous, ?current]),
          child: Column(
            key: ValueKey(filter.value),
            children: [
              if (shown.isEmpty)
                const EmptyState(
                  icon: FLucideIcons.fileX,
                  title: 'No courses yet',
                )
              else
                for (final entry in _bySession(shown).entries) ...[
                  SectionHeader(
                    title: entry.key.isEmpty
                        ? 'Other'
                        : _sessionLabel(entry.key),
                    // GPA of the whole session, not just filtered rows.
                    trailing: Text(switch (_sessionGpa(
                      history.records.where(
                        (r) => r.examMonth.trim() == entry.key,
                      ),
                    )) {
                      final gpa? => 'GPA ${gpa.toStringAsFixed(2)}',
                      null => '',
                    }),
                  ),
                  for (final (i, r) in entry.value.indexed)
                    Padding(
                      padding: const .only(bottom: Space.sm),
                      child: EnterFade(
                        index: i,
                        child: _CourseCard(record: r),
                      ),
                    ),
                ],
            ],
          ),
        ),
        DataUpdatedFooter(updateTime: history.updateTime.toInt()),
      ],
    );
  }
}

/// CGPA front and centre, credits beside it, and one bar for the grade mix.
class _CgpaHero extends StatelessWidget {
  const _CgpaHero({required this.history});

  final GradeHistoryData history;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final c = history.cgpa;
    final counts =
        {
            'S': c.sGrades,
            'A': c.aGrades,
            'B': c.bGrades,
            'C': c.cGrades,
            'D': c.dGrades,
            'E': c.eGrades,
            'F': c.fGrades,
            'N': c.nGrades,
          }.map((k, v) => MapEntry(k, int.tryParse(v.trim()) ?? 0))
          ..removeWhere((_, v) => v == 0);
    final total = counts.values.fold<int>(0, (a, b) => a + b);
    final cgpa = double.tryParse(c.cgpa.trim());
    final trend = [
      for (final e in _bySession(history.records).entries)
        if (_sessionGpa(e.value) case final gpa?) (_shortSession(e.key), gpa),
    ];

    return Surface(
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          Text(
            [
              history.student.name.trim(),
              history.student.regNo.trim(),
            ].where((p) => p.isNotEmpty).join(' · '),
            maxLines: 1,
            overflow: .ellipsis,
            style: typography.body.xs.copyWith(color: colors.mutedForeground),
          ),
          const SizedBox(height: Space.md),
          Row(
            crossAxisAlignment: .end,
            children: [
              Column(
                crossAxisAlignment: .start,
                children: [
                  Text(
                    'CGPA',
                    style: typography.body.xs.copyWith(
                      fontWeight: .w600,
                      letterSpacing: 0.6,
                      color: colors.mutedForeground,
                    ),
                  ),
                  cgpa == null
                      ? Text(c.cgpa, style: typography.display.xl3)
                      : CountUp(
                          value: cgpa,
                          decimals: 2,
                          style: typography.display.xl3.copyWith(
                            height: 1.1,
                            fontWeight: .w600,
                            letterSpacing: -0.5,
                            color: colors.foreground,
                          ),
                        ),
                ],
              ),
              const Spacer(),
              _Stat(label: 'CREDITS EARNED', value: c.creditsEarned),
              const SizedBox(width: Space.lg),
              _Stat(label: 'REGISTERED', value: c.creditsRegistered),
            ],
          ),
          if (total > 0) ...[
            const SizedBox(height: Space.lg),
            ClipRRect(
              borderRadius: .circular(Radii.pill),
              child: SizedBox(
                height: 10,
                child: Row(
                  children: [
                    for (final e in counts.entries)
                      Expanded(
                        flex: e.value,
                        child: Container(
                          margin: const .only(right: 2),
                          color: gradeTone(context, e.key).base,
                        ),
                      ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: Space.sm),
            Wrap(
              spacing: Space.md,
              runSpacing: Space.xs,
              children: [
                for (final e in counts.entries)
                  Row(
                    mainAxisSize: .min,
                    children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: gradeTone(context, e.key).base,
                          borderRadius: .circular(2),
                        ),
                      ),
                      const SizedBox(width: Space.xs),
                      Text(
                        '${e.key} ${e.value}',
                        style: typography.body.xs.nums(
                          color: colors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ],
          if (trend.length >= 2) ...[
            const SizedBox(height: Space.lg),
            Text(
              'GPA BY SEMESTER',
              style: typography.body.xs.copyWith(
                fontSize: 10,
                fontWeight: .w600,
                letterSpacing: 0.5,
                color: colors.mutedForeground,
              ),
            ),
            const SizedBox(height: Space.sm),
            _GpaTrend(points: trend),
          ],
        ],
      ),
    );
  }
}

/// Semester GPAs as a line with a dot and value per semester.
class _GpaTrend extends StatelessWidget {
  const _GpaTrend({required this.points});

  final List<(String, double)> points;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final values = points.map((p) => p.$2);
    final lo = (values.reduce(math.min) - 0.5).clamp(0.0, 10.0);
    final hi = (values.reduce(math.max) + 0.3).clamp(0.0, 10.0);
    const chartHeight = 56.0;

    return Column(
      children: [
        SizedBox(
          height: chartHeight + 18,
          child: LayoutBuilder(
            builder: (context, c) {
              final step = c.maxWidth / points.length;
              Offset at(int i) => Offset(
                step * (i + 0.5),
                18 + chartHeight * (1 - (points[i].$2 - lo) / (hi - lo)),
              );
              return Stack(
                clipBehavior: .none,
                children: [
                  Positioned.fill(
                    child: CustomPaint(
                      painter: _TrendPainter(
                        [for (var i = 0; i < points.length; i++) at(i)],
                        colors.app.accent,
                        colors.card,
                      ),
                    ),
                  ),
                  for (var i = 0; i < points.length; i++)
                    Positioned(
                      left: at(i).dx - 24,
                      top: at(i).dy - 20,
                      width: 48,
                      child: Text(
                        points[i].$2.toStringAsFixed(2),
                        textAlign: .center,
                        style: typography.body.xs.nums(
                          fontSize: 10.5,
                          fontWeight: .w600,
                          color: colors.foreground,
                        ),
                      ),
                    ),
                ],
              );
            },
          ),
        ),
        const SizedBox(height: Space.xs),
        Row(
          children: [
            for (final p in points)
              Expanded(
                child: Text(
                  p.$1,
                  textAlign: .center,
                  maxLines: 1,
                  style: typography.body.xs.copyWith(
                    fontSize: 10,
                    color: colors.mutedForeground,
                  ),
                ),
              ),
          ],
        ),
      ],
    );
  }
}

class _TrendPainter extends CustomPainter {
  _TrendPainter(this.points, this.color, this.hole);

  final List<Offset> points;
  final Color color;
  final Color hole;

  @override
  void paint(Canvas canvas, Size size) {
    final line = Paint()
      ..color = color
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    canvas.drawPath(Path()..addPolygon(points, false), line);
    for (final p in points) {
      canvas.drawCircle(p, 4.5, Paint()..color = hole);
      canvas.drawCircle(p, 3.5, Paint()..color = color);
    }
  }

  @override
  bool shouldRepaint(_TrendPainter old) =>
      old.points != points || old.color != color;
}

class _Stat extends StatelessWidget {
  const _Stat({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final number = double.tryParse(value.trim());
    return Column(
      crossAxisAlignment: .end,
      children: [
        Text(
          label,
          style: typography.body.xs.copyWith(
            fontSize: 10,
            fontWeight: .w600,
            letterSpacing: 0.5,
            color: colors.mutedForeground,
          ),
        ),
        Text(
          number == null
              ? value
              : number == number.roundToDouble()
              ? number.toStringAsFixed(0)
              : number.toString(),
          style: typography.body.lg.nums(
            fontWeight: .w500,
            color: colors.foreground,
          ),
        ),
      ],
    );
  }
}

class _FilterChip extends StatelessWidget {
  const _FilterChip({
    required this.label,
    required this.selected,
    required this.onPress,
    this.count,
    this.tone,
  });

  final String label;
  final int? count;
  final Tone? tone;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Padding(
      padding: const .only(right: Space.sm),
      child: PressScale(
        scale: 0.95,
        semanticsLabel: 'Show grade $label',
        onPress: onPress,
        child: AnimatedContainer(
          duration: Motion.medium,
          padding: const .symmetric(
            horizontal: Space.md,
            vertical: Space.sm - 1,
          ),
          decoration: BoxDecoration(
            color: selected ? colors.primary : colors.card,
            borderRadius: .circular(Radii.pill),
            border: .all(color: selected ? colors.primary : colors.border),
          ),
          child: Row(
            mainAxisSize: .min,
            children: [
              if (tone != null) ...[
                Container(
                  width: 8,
                  height: 8,
                  decoration: BoxDecoration(
                    color: tone!.base,
                    borderRadius: .circular(2),
                  ),
                ),
                const SizedBox(width: Space.xs + 2),
              ],
              Text(
                count == null ? label : '$label  $count',
                style: typography.body.sm.copyWith(
                  fontWeight: .w500,
                  color: selected
                      ? colors.primaryForeground
                      : colors.foreground,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CourseCard extends HookWidget {
  const _CourseCard({required this.record});

  final GradeHistoryRecord record;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final expanded = useState(false);
    final credits = double.tryParse(record.credits.trim());
    final components = record.attempts
        .where((a) => a.courseType.trim() != record.courseType.trim())
        .toList();

    return Surface(
      padding: .zero,
      child: Column(
        children: [
          PressScale(
            scale: 0.99,
            semanticsLabel: '${record.courseTitle}, grade ${record.grade}',
            onPress: () => expanded.value = !expanded.value,
            child: Padding(
              padding: const .all(Space.md + 2),
              child: Row(
                children: [
                  GradeBadge(grade: record.grade),
                  const SizedBox(width: Space.md),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: .start,
                      children: [
                        Text(
                          record.courseTitle,
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
                          [
                            record.courseCode,
                            record.courseDistribution,
                          ].where((p) => p.trim().isNotEmpty).join(' · '),
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (credits != null) ...[
                    const SizedBox(width: Space.md),
                    Column(
                      crossAxisAlignment: .end,
                      children: [
                        Text(
                          credits == credits.roundToDouble()
                              ? credits.toStringAsFixed(0)
                              : '$credits',
                          style: typography.body.lg.nums(
                            height: 1.1,
                            fontWeight: .w600,
                            color: colors.foreground,
                          ),
                        ),
                        Text(
                          credits == 1 ? 'credit' : 'credits',
                          style: typography.body.xs.copyWith(
                            fontSize: 10,
                            color: colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(width: Space.sm),
                  ],
                  // Space kept when there's nothing to expand, so credits line up.
                  if (components.isEmpty)
                    const SizedBox(width: 16)
                  else
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
          AnimatedSize(
            duration: Motion.medium,
            curve: Curves.easeOutCubic,
            alignment: .topCenter,
            child: expanded.value && components.isNotEmpty
                ? Column(
                    children: [
                      Container(height: 1, color: colors.border),
                      Padding(
                        padding: const .fromLTRB(
                          Space.md + 2,
                          Space.sm,
                          Space.md + 2,
                          Space.md,
                        ),
                        child: Column(
                          children: [
                            for (final a in components)
                              Padding(
                                padding: const .symmetric(vertical: Space.xs),
                                child: Row(
                                  children: [
                                    Expanded(
                                      child: Text(
                                        _componentLabel(a.courseType),
                                        style: typography.body.sm.copyWith(
                                          color: colors.foreground,
                                        ),
                                      ),
                                    ),
                                    Text(
                                      'Declared ${a.resultDeclared}',
                                      style: typography.body.xs.copyWith(
                                        color: colors.mutedForeground,
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                          ],
                        ),
                      ),
                    ],
                  )
                : const SizedBox(width: double.infinity),
          ),
        ],
      ),
    );
  }

  static String _componentLabel(String type) =>
      switch (type.trim().toUpperCase()) {
        'ETH' || 'TH' => 'Theory component',
        'ELA' || 'LO' => 'Lab component',
        'EPJ' || 'PJT' => 'Project component',
        _ => type,
      };
}
