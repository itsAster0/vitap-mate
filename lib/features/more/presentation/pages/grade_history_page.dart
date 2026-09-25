import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/grade_badge.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

const _gradeOrder = ['S', 'A', 'B', 'C', 'D', 'E', 'F', 'N'];

class GradeHistoryPage extends HookConsumerWidget {
  const GradeHistoryPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data = ref.watch(gradeHistoryProvider);

    Future<void> reload() async {
      try {
        await ref.read(gradeHistoryProvider.notifier).refresh();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(gradeHistoryProvider.notifier).refresh().catchError((e, st) {
          log('auto refresh failed: $e', stackTrace: st);
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
          padding: EdgeInsets.all(Space.sm),
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
            variant: FButtonVariant.outline,
            mainAxisSize: MainAxisSize.min,
            onPress: reload,
            child: const Text('Try again'),
          ),
        ),
        data: (history) => RefreshIndicator(
          key: const ValueKey('data'),
          onRefresh: reload,
          backgroundColor: context.theme.colors.background,
          color: context.theme.colors.foreground,
          child: _HistoryView(history: history),
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
      padding: const EdgeInsets.fromLTRB(
        Space.sm,
        Space.sm,
        Space.sm,
        Space.lg,
      ),
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        _CgpaHero(history: history),
        if (history.records.isNotEmpty) ...[
          SectionHeader(
            title: 'Courses',
            trailing: Text('${shown.length} of ${history.records.length}'),
          ),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
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
          layoutBuilder: (current, previous) => Stack(
            alignment: Alignment.topCenter,
            children: [...previous, ?current],
          ),
          child: Column(
            key: ValueKey(filter.value),
            children: [
              if (shown.isEmpty)
                const EmptyState(
                  icon: FLucideIcons.fileX,
                  title: 'No courses yet',
                )
              else
                for (final (i, r) in shown.indexed)
                  Padding(
                    padding: const EdgeInsets.only(bottom: Space.sm),
                    child: EnterFade(
                      index: i,
                      child: _CourseCard(record: r),
                    ),
                  ),
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

    return Surface(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            [
              history.student.name.trim(),
              history.student.regNo.trim(),
            ].where((p) => p.isNotEmpty).join(' · '),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: typography.body.xs.copyWith(color: colors.mutedForeground),
          ),
          const SizedBox(height: Space.md),
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'CGPA',
                    style: typography.body.xs.copyWith(
                      fontWeight: FontWeight.w600,
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
                            fontWeight: FontWeight.w600,
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
              borderRadius: BorderRadius.circular(Radii.pill),
              child: SizedBox(
                height: 10,
                child: Row(
                  children: [
                    for (final e in counts.entries)
                      Expanded(
                        flex: e.value,
                        child: Container(
                          margin: const EdgeInsets.only(right: 2),
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
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: gradeTone(context, e.key).base,
                          borderRadius: BorderRadius.circular(2),
                        ),
                      ),
                      const SizedBox(width: Space.xs),
                      Text(
                        '${e.key} ${e.value}',
                        style: typography.body.xs.copyWith(
                          color: colors.mutedForeground,
                          fontFeatures: const [FontFeature.tabularFigures()],
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
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        Text(
          label,
          style: typography.body.xs.copyWith(
            fontSize: 10,
            fontWeight: FontWeight.w600,
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
          style: typography.body.lg.copyWith(
            fontWeight: FontWeight.w500,
            color: colors.foreground,
            fontFeatures: const [FontFeature.tabularFigures()],
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
      padding: const EdgeInsets.only(right: Space.sm),
      child: PressScale(
        scale: 0.95,
        semanticsLabel: 'Show grade $label',
        onPress: onPress,
        child: AnimatedContainer(
          duration: Motion.medium,
          padding: const EdgeInsets.symmetric(
            horizontal: Space.md,
            vertical: Space.sm - 1,
          ),
          decoration: BoxDecoration(
            color: selected ? colors.primary : colors.card,
            borderRadius: BorderRadius.circular(Radii.pill),
            border: Border.all(
              color: selected ? colors.primary : colors.border,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (tone != null) ...[
                Container(
                  width: 8,
                  height: 8,
                  decoration: BoxDecoration(
                    color: tone!.base,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
                const SizedBox(width: Space.xs + 2),
              ],
              Text(
                count == null ? label : '$label  $count',
                style: typography.body.sm.copyWith(
                  fontWeight: FontWeight.w500,
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
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          PressScale(
            scale: 0.99,
            semanticsLabel: '${record.courseTitle}, grade ${record.grade}',
            onPress: () => expanded.value = !expanded.value,
            child: Padding(
              padding: const EdgeInsets.all(Space.md + 2),
              child: Row(
                children: [
                  GradeBadge(grade: record.grade),
                  const SizedBox(width: Space.md),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          record.courseTitle,
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
                          [
                            record.courseCode,
                            if (credits != null)
                              '${credits == credits.roundToDouble() ? credits.toStringAsFixed(0) : credits} credits',
                          ].join(' · '),
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                        Text(
                          [
                            record.examMonth.replaceAll('-', ' '),
                            record.courseDistribution,
                          ].where((p) => p.trim().isNotEmpty).join(' · '),
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (components.isNotEmpty)
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
            alignment: Alignment.topCenter,
            child: expanded.value && components.isNotEmpty
                ? Column(
                    children: [
                      Container(height: 1, color: colors.border),
                      Padding(
                        padding: const EdgeInsets.fromLTRB(
                          Space.md + 2,
                          Space.sm,
                          Space.md + 2,
                          Space.md,
                        ),
                        child: Column(
                          children: [
                            for (final a in components)
                              Padding(
                                padding: const EdgeInsets.symmetric(
                                  vertical: Space.xs,
                                ),
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
