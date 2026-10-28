import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/features/exams/exams.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';

class ExamSchedulePage extends HookConsumerWidget {
  const ExamSchedulePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> update() async {
      try {
        await ref.read(examScheduleProvider.notifier).refresh();
      } catch (e) {
        Log.info('features.exams', "$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(examScheduleProvider.notifier).refresh().catchError((e, st) {
          Log.info('features.exams', 'auto refresh failed: $e');
        });
      });
      return null;
    }, const []);

    final examData = ref.watch(examScheduleProvider);

    return AnimatedSwitcher(
      duration: Motion.medium,
      child: examData.when(
        skipLoadingOnRefresh: true,
        skipLoadingOnReload: true,
        data: (data) => ScreenRefresh(
          onRefresh: update,
          tasks: const ['vtop_fetchSchedule'],
          child: RefreshIndicator(
            key: const ValueKey('data'),
            onRefresh: update,
            backgroundColor: context.theme.colors.background,
            color: context.theme.colors.foreground,
            child: data.exams.isEmpty
                ? ListView(
                    physics: const AlwaysScrollableScrollPhysics(),
                    children: const [
                      EmptyState(
                        icon: FLucideIcons.calendarX,
                        title: 'No exam schedule yet',
                        message: 'It shows up here once VTOP publishes it.',
                      ),
                    ],
                  )
                : _ExamsView(
                    exams: data.exams,
                    updateTime: data.updateTime.toInt(),
                  ),
          ),
        ),
        error: (e, _) => EmptyState(
          key: const ValueKey('error'),
          icon: FLucideIcons.cloudAlert,
          title: "Couldn't load exams",
          message: commonErrorMessage(e),
          action: FButton(
            variant: .outline,
            mainAxisSize: .min,
            onPress: update,
            child: const Text('Try again'),
          ),
        ),
        loading: () => const Padding(
          key: ValueKey('loading'),
          padding: .all(Space.sm),
          child: Column(
            children: [
              Skeleton(height: 40, radius: Radii.md),
              SizedBox(height: Space.lg),
              SkeletonList(count: 4, height: 104),
            ],
          ),
        ),
      ),
    );
  }
}

class _ExamsView extends HookWidget {
  const _ExamsView({required this.exams, required this.updateTime});

  final List<PerExamScheduleRecord> exams;
  final int updateTime;

  @override
  Widget build(BuildContext context) {
    // CAT1, CAT2, FAT… in natural order regardless of how VTOP lists them.
    final types = [...exams]
      ..sort((a, b) => _typeRank(a.examType).compareTo(_typeRank(b.examType)));
    final now = DateTime.now();

    // Open on the next exam type — one with an upcoming or not-yet-announced
    // date — else the latest with dates.
    int initialIndex() {
      final next = types.indexWhere(
        (t) => t.records.any(
          (e) =>
              examDayOf(e) == null || (examStartOf(e)?.isAfter(now) ?? false),
        ),
      );
      if (next >= 0) return next;
      final latest = types.lastIndexWhere(
        (t) => t.records.any((e) => examDayOf(e) != null),
      );
      return latest >= 0 ? latest : 0;
    }

    final selected = useState<int>(useMemoized(initialIndex));
    final index = selected.value.clamp(0, types.length - 1);

    return ListView(
      padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.lg),
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        if (types.length > 1) ...[
          Segmented<int>(
            value: index,
            onChanged: (value) => selected.value = value,
            segments: [
              for (final (i, t) in types.indexed) (i, examName(t.examType)),
            ],
          ),
          const SizedBox(height: Space.xs),
        ],
        AnimatedSwitcher(
          duration: Motion.medium,
          switchInCurve: const Interval(0.3, 1, curve: Curves.easeOut),
          switchOutCurve: const Interval(0.7, 1, curve: Curves.easeIn),
          layoutBuilder: (current, previous) =>
              Stack(alignment: .topCenter, children: [...previous, ?current]),
          child: _Timeline(key: ValueKey(index), exams: types[index].records),
        ),
        DataUpdatedFooter(updateTime: updateTime),
      ],
    );
  }

  static int _typeRank(String type) =>
      switch (type.toUpperCase().replaceAll(' ', '')) {
        final t when t.startsWith('CAT') => int.tryParse(t.substring(3)) ?? 5,
        final t when t.contains('FAT') => 10,
        _ => 20,
      };
}

/// Exams grouped by day, with the free days between them called out.
class _Timeline extends StatelessWidget {
  const _Timeline({super.key, required this.exams});

  final List<ExamScheduleRecord> exams;

  @override
  Widget build(BuildContext context) {
    final byDay = <DateTime?, List<ExamScheduleRecord>>{};
    for (final exam in exams) {
      byDay.putIfAbsent(examDayOf(exam), () => []).add(exam);
    }
    DateTime startOf(ExamScheduleRecord e) => examStartOf(e) ?? DateTime(9999);
    for (final list in byDay.values) {
      list.sort((a, b) => startOf(a).compareTo(startOf(b)));
    }
    final days = byDay.keys.whereType<DateTime>().toList()..sort();
    final undated = byDay[null] ?? const [];
    var row = 0;

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        for (final (i, day) in days.indexed) ...[
          // Past days carry their date inside each compact row, so they get
          // no header or prep gap.
          if (!_isPastDay(day)) ...[
            if (i > 0 && !_isPastDay(days[i - 1]))
              _GapRow(days: day.difference(days[i - 1]).inDays - 1),
            _DayHeader(day: day, count: byDay[day]!.length),
          ] else if (i == 0)
            const SizedBox(height: Space.md),
          for (final exam in byDay[day]!)
            Padding(
              padding: const .only(bottom: Space.sm),
              child: EnterFade(
                index: row++,
                child: _ExamCard(exam: exam, day: day),
              ),
            ),
        ],
        if (undated.isNotEmpty) ...[
          SectionHeader(
            title: 'Date not announced',
            trailing: Text('${undated.length} courses'),
          ),
          Surface(
            padding: .zero,
            child: Column(
              children: [
                for (final (i, exam) in undated.indexed) ...[
                  if (i > 0)
                    Container(height: 1, color: context.theme.colors.border),
                  _CourseLine(exam: exam),
                ],
              ],
            ),
          ),
        ],
      ],
    );
  }
}

class _DayHeader extends StatelessWidget {
  const _DayHeader({required this.day, required this.count});

  final DateTime day;
  final int count;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final muted = typography.body.xs.copyWith(color: colors.mutedForeground);
    final (label, tone) = _relative(context, day);
    return Padding(
      padding: const .fromLTRB(2, Space.lg, 2, Space.sm),
      child: Row(
        children: [
          Text(
            DateFormat('EEE d MMM').format(day).toUpperCase(),
            style: typography.body.sm.copyWith(
              fontWeight: .w700,
              letterSpacing: 0.4,
              color: colors.foreground,
            ),
          ),
          const SizedBox(width: Space.sm),
          if (tone != null)
            ToneBadge(label: label, tone: tone)
          else
            Text(label, style: muted),
          const Spacer(),
          if (count > 1) Text('$count exams', style: muted),
        ],
      ),
    );
  }
}

DateTime _dateOnly(DateTime d) => DateTime(d.year, d.month, d.day);

bool _isPastDay(DateTime day) =>
    _dateOnly(day).isBefore(_dateOnly(DateTime.now()));

/// Day number over a short weekday, for rows that carry their own date.
class _DateColumn extends StatelessWidget {
  const _DateColumn({required this.day});

  final DateTime day;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return SizedBox(
      width: 34,
      child: Column(
        children: [
          Text(
            '${day.day}',
            style: typography.body.lg.nums(
              height: 1.1,
              fontWeight: .w700,
              color: colors.mutedForeground,
            ),
          ),
          Text(
            DateFormat('EEE').format(day).toUpperCase(),
            style: typography.body.xs.copyWith(
              fontSize: 10,
              letterSpacing: 0.4,
              color: colors.mutedForeground,
            ),
          ),
        ],
      ),
    );
  }
}

/// "done", "TODAY", "TOMORROW", "IN 3 DAYS" – with a tone for upcoming days.
(String, Tone?) _relative(BuildContext context, DateTime day) {
  final palette = context.theme.colors.app;
  return switch (_dateOnly(day).difference(_dateOnly(DateTime.now())).inDays) {
    < 0 => ('done', null),
    0 => ('TODAY', palette.danger),
    1 => ('TOMORROW', palette.warning),
    final diff => ('IN $diff DAYS', palette.accentTone),
  };
}

class _GapRow extends StatelessWidget {
  const _GapRow({required this.days});

  final int days;

  @override
  Widget build(BuildContext context) {
    if (days <= 0) return const SizedBox.shrink();
    final colors = context.theme.colors;
    final line = Expanded(child: Container(height: 1, color: colors.border));
    return Padding(
      padding: const .only(top: Space.sm),
      child: Row(
        children: [
          line,
          Padding(
            padding: const .symmetric(horizontal: Space.sm),
            child: Row(
              children: [
                Icon(
                  FLucideIcons.bookOpenCheck,
                  size: 12,
                  color: colors.mutedForeground,
                ),
                const SizedBox(width: Space.xs),
                Text(
                  '$days ${days == 1 ? 'day' : 'days'} to prepare',
                  style: context.theme.typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          line,
        ],
      ),
    );
  }
}

class _ExamCard extends HookWidget {
  const _ExamCard({required this.exam, required this.day});

  final ExamScheduleRecord exam;
  final DateTime day;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final start = examStartOf(exam);
    final done = start != null && start.isBefore(DateTime.now());
    final startClock = formatClock(exam.examTime.split('-').first, context);
    final expanded = useState(false);
    void toggle() => expanded.value = !expanded.value;

    // Finished exams shrink to one row so the upcoming ones stand out; tap
    // to see the full card again.
    if (done && !expanded.value) {
      return Surface(
        onPress: toggle,
        semanticsLabel: '${exam.courseName}, finished, show details',
        padding: const .symmetric(horizontal: Space.md + 2, vertical: Space.md),
        child: Row(
          children: [
            _isPastDay(day)
                ? _DateColumn(day: day)
                : Icon(
                    FLucideIcons.circleCheck,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
            const SizedBox(width: Space.md),
            Expanded(
              child: Column(
                crossAxisAlignment: .start,
                children: [
                  Text(
                    exam.courseName,
                    maxLines: 1,
                    overflow: .ellipsis,
                    style: typography.body.sm.copyWith(
                      fontWeight: .w500,
                      color: colors.mutedForeground,
                    ),
                  ),
                  if (hasValue(exam.venue))
                    Text(
                      exam.venue.trim(),
                      style: typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                ],
              ),
            ),
            const SizedBox(width: Space.md),
            Text(
              startClock,
              style: typography.body.xs.nums(color: colors.mutedForeground),
            ),
          ],
        ),
      );
    }

    final factRows = [
      [
        if (hasValue(exam.venue))
          _Fact(label: 'VENUE', value: exam.venue.trim()),
        if (hasValue(exam.seatLocation))
          _Fact(label: 'SEAT', value: _seatLabel(exam.seatLocation.trim())),
      ],
      [
        if (hasValue(exam.seatNo))
          _Fact(label: 'SEAT NO.', value: exam.seatNo.trim()),
        if (hasValue(exam.reportingTime))
          _Fact(
            label: 'REPORT',
            value: formatClock(exam.reportingTime, context),
          ),
      ],
    ].where((facts) => facts.isNotEmpty);

    return Surface(
      onPress: done ? toggle : null,
      padding: const .all(Space.md + 2),
      child: Column(
        crossAxisAlignment: .start,
        children: [
          Row(
            crossAxisAlignment: .start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: .start,
                  children: [
                    if (_isPastDay(day)) ...[
                      ToneBadge(
                        label: DateFormat(
                          'EEE d MMM',
                        ).format(day).toUpperCase(),
                        icon: FLucideIcons.calendar,
                        tone: colors.app.accentTone,
                      ),
                      const SizedBox(height: Space.sm),
                    ],
                    Text(
                      exam.courseName,
                      maxLines: 2,
                      overflow: .ellipsis,
                      style: typography.body.md.copyWith(
                        height: 1.25,
                        fontWeight: .w600,
                        color: colors.foreground,
                      ),
                    ),
                    Text(
                      exam.courseCode,
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
                    startClock,
                    style: typography.body.md.nums(
                      fontWeight: .w500,
                      color: colors.foreground,
                    ),
                  ),
                  if (exam.examTime.contains('-'))
                    Text(
                      'to ${formatClock(exam.examTime.split('-').last, context)}',
                      style: typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                ],
              ),
            ],
          ),
          for (final (i, facts) in factRows.indexed) ...[
            SizedBox(height: i == 0 ? Space.md : Space.sm),
            Row(
              children: [
                for (final (j, fact) in facts.indexed) ...[
                  if (j > 0) const SizedBox(width: Space.sm),
                  fact,
                ],
              ],
            ),
          ],
        ],
      ),
    );
  }

  /// "R6C6" → "R6 · C6" for readability; anything else as-is.
  static String _seatLabel(String raw) {
    final m = RegExp(r'^R(\d+)C(\d+)$', caseSensitive: false).firstMatch(raw);
    return m == null ? raw : 'R${m.group(1)} · C${m.group(2)}';
  }
}

class _Fact extends StatelessWidget {
  const _Fact({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Expanded(
      child: Column(
        crossAxisAlignment: .start,
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
          const SizedBox(height: 2),
          Text(
            value,
            maxLines: 1,
            overflow: .ellipsis,
            style: typography.body.sm.nums(
              fontWeight: .w600,
              color: colors.foreground,
            ),
          ),
        ],
      ),
    );
  }
}

class _CourseLine extends StatelessWidget {
  const _CourseLine({required this.exam});

  final ExamScheduleRecord exam;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Padding(
      padding: const .symmetric(horizontal: Space.md + 2, vertical: Space.md),
      child: Row(
        children: [
          Expanded(
            child: Text(
              exam.courseName,
              maxLines: 1,
              overflow: .ellipsis,
              style: typography.body.sm.copyWith(color: colors.foreground),
            ),
          ),
          const SizedBox(width: Space.md),
          Text(
            exam.courseCode,
            style: typography.body.xs.copyWith(color: colors.mutedForeground),
          ),
        ],
      ),
    );
  }
}
