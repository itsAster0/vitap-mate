import 'dart:developer';

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/more/domain/exam_time.dart';
import 'package:vitapmate/features/timetable/presentation/utils/time_format.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamSchedulePage extends HookConsumerWidget {
  const ExamSchedulePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> update() async {
      try {
        await ref.read(examScheduleProvider.notifier).updatexamschedule();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(examScheduleProvider.notifier).updatexamschedule().catchError((
          e,
          st,
        ) {
          log('auto refresh failed: $e', stackTrace: st);
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
            variant: FButtonVariant.outline,
            mainAxisSize: MainAxisSize.min,
            onPress: update,
            child: const Text('Try again'),
          ),
        ),
        loading: () => const Padding(
          key: ValueKey('loading'),
          padding: EdgeInsets.all(Space.sm),
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
      for (final (i, t) in types.indexed) {
        if (t.records.any(
          (e) =>
              examDayOf(e) == null || (examStartOf(e)?.isAfter(now) ?? false),
        )) {
          return i;
        }
      }
      for (var i = types.length - 1; i >= 0; i--) {
        if (types[i].records.any((e) => examDayOf(e) != null)) return i;
      }
      return 0;
    }

    final selected = useState<int>(useMemoized(initialIndex));
    final index = selected.value.clamp(0, types.length - 1);

    return ListView(
      padding: const EdgeInsets.fromLTRB(
        Space.sm,
        Space.sm,
        Space.sm,
        Space.lg,
      ),
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        if (types.length > 1) ...[
          Segmented<int>(
            value: index,
            onChanged: (value) => selected.value = value,
            segments: [
              for (final (i, t) in types.indexed) (i, t.examType.trim()),
            ],
          ),
          const SizedBox(height: Space.xs),
        ],
        AnimatedSwitcher(
          duration: Motion.medium,
          switchInCurve: const Interval(0.3, 1, curve: Curves.easeOut),
          switchOutCurve: const Interval(0.7, 1, curve: Curves.easeIn),
          layoutBuilder: (current, previous) => Stack(
            alignment: Alignment.topCenter,
            children: [...previous, ?current],
          ),
          child: _Timeline(key: ValueKey(index), exams: types[index].records),
        ),
        DataUpdatedFooter(updateTime: updateTime),
      ],
    );
  }

  static int _typeRank(String type) {
    final t = type.toUpperCase().replaceAll(' ', '');
    if (t.startsWith('CAT')) {
      return int.tryParse(t.substring(3)) ?? 5;
    }
    if (t.contains('FAT')) return 10;
    return 20;
  }
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
    for (final list in byDay.values) {
      list.sort(
        (a, b) => (examStartOf(a) ?? DateTime(9999)).compareTo(
          examStartOf(b) ?? DateTime(9999),
        ),
      );
    }
    final days = byDay.keys.whereType<DateTime>().toList()..sort();
    final undated = byDay[null] ?? const [];
    var row = 0;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
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
              padding: const EdgeInsets.only(bottom: Space.sm),
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
            padding: EdgeInsets.zero,
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
    final (label, tone) = _relative(context, day);
    return Padding(
      padding: const EdgeInsets.fromLTRB(2, Space.lg, 2, Space.sm),
      child: Row(
        children: [
          Text(
            DateFormat('EEE d MMM').format(day).toUpperCase(),
            style: typography.body.sm.copyWith(
              fontWeight: FontWeight.w700,
              letterSpacing: 0.4,
              color: colors.foreground,
            ),
          ),
          const SizedBox(width: Space.sm),
          if (tone != null)
            ToneBadge(label: label, tone: tone)
          else
            Text(
              label,
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
          const Spacer(),
          if (count > 1)
            Text(
              '$count exams',
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
        ],
      ),
    );
  }
}

bool _isPastDay(DateTime day) {
  final now = DateTime.now();
  return DateTime(
    day.year,
    day.month,
    day.day,
  ).isBefore(DateTime(now.year, now.month, now.day));
}

/// Day number over a short weekday, for rows that carry their own date.
class _DateColumn extends StatelessWidget {
  const _DateColumn({required this.day, this.muted = false});

  final DateTime day;
  final bool muted;

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
            style: typography.body.lg.copyWith(
              height: 1.1,
              fontWeight: FontWeight.w700,
              color: muted ? colors.mutedForeground : colors.foreground,
              fontFeatures: const [FontFeature.tabularFigures()],
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
  final now = DateTime.now();
  final today = DateTime(now.year, now.month, now.day);
  final diff = DateTime(day.year, day.month, day.day).difference(today).inDays;
  if (diff < 0) return ('done', null);
  if (diff == 0) return ('TODAY', palette.danger);
  if (diff == 1) return ('TOMORROW', palette.warning);
  return ('IN $diff DAYS', palette.accentTone);
}

class _GapRow extends StatelessWidget {
  const _GapRow({required this.days});

  final int days;

  @override
  Widget build(BuildContext context) {
    if (days <= 0) return const SizedBox.shrink();
    final colors = context.theme.colors;
    return Padding(
      padding: const EdgeInsets.only(top: Space.sm),
      child: Row(
        children: [
          Expanded(child: Container(height: 1, color: colors.border)),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: Space.sm),
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
          Expanded(child: Container(height: 1, color: colors.border)),
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
    final hasLocation = hasValue(exam.venue) || hasValue(exam.seatLocation);
    final hasNumberOrReport =
        hasValue(exam.seatNo) || hasValue(exam.reportingTime);
    final expanded = useState(false);
    void toggle() => expanded.value = !expanded.value;

    // Finished exams shrink to one row so the upcoming ones stand out; tap
    // to see the full card again.
    if (done && !expanded.value) {
      return Surface(
        onPress: toggle,
        semanticsLabel: '${exam.courseName}, finished, show details',
        padding: const EdgeInsets.symmetric(
          horizontal: Space.md + 2,
          vertical: Space.md,
        ),
        child: Row(
          children: [
            _isPastDay(day)
                ? _DateColumn(day: day, muted: true)
                : Icon(
                    FLucideIcons.circleCheck,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
            const SizedBox(width: Space.md),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    exam.courseName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: typography.body.sm.copyWith(
                      fontWeight: FontWeight.w500,
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
              formatClock(exam.examTime.split('-').first, context),
              style: typography.body.xs.copyWith(
                color: colors.mutedForeground,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ],
        ),
      );
    }

    // A finished exam only reaches here when opened, so show it at full
    // strength.
    return AnimatedOpacity(
      duration: Motion.medium,
      opacity: 1,
      child: Surface(
        onPress: done ? toggle : null,
        padding: const EdgeInsets.all(Space.md + 2),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
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
                        overflow: TextOverflow.ellipsis,
                        style: typography.body.md.copyWith(
                          height: 1.25,
                          fontWeight: FontWeight.w600,
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
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    Text(
                      formatClock(exam.examTime.split('-').first, context),
                      style: typography.body.md.copyWith(
                        fontWeight: FontWeight.w500,
                        color: colors.foreground,
                        fontFeatures: const [FontFeature.tabularFigures()],
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
            if (hasLocation || hasNumberOrReport) ...[
              const SizedBox(height: Space.md),
              if (hasLocation)
                Row(
                  children: [
                    if (hasValue(exam.venue))
                      _Fact(label: 'VENUE', value: exam.venue.trim()),
                    if (hasValue(exam.venue) && hasValue(exam.seatLocation))
                      const SizedBox(width: Space.sm),
                    if (hasValue(exam.seatLocation))
                      _Fact(
                        label: 'SEAT',
                        value: _seatLabel(exam.seatLocation.trim()),
                      ),
                  ],
                ),
              if (hasLocation && hasNumberOrReport)
                const SizedBox(height: Space.sm),
              if (hasNumberOrReport)
                Row(
                  children: [
                    if (hasValue(exam.seatNo))
                      _Fact(label: 'SEAT NO.', value: exam.seatNo.trim()),
                    if (hasValue(exam.seatNo) && hasValue(exam.reportingTime))
                      const SizedBox(width: Space.sm),
                    if (hasValue(exam.reportingTime))
                      _Fact(
                        label: 'REPORT',
                        value: formatClock(exam.reportingTime, context),
                      ),
                  ],
                ),
            ],
          ],
        ),
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
        crossAxisAlignment: CrossAxisAlignment.start,
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
          const SizedBox(height: 2),
          Text(
            value,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
              color: colors.foreground,
              fontFeatures: const [FontFeature.tabularFigures()],
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
      padding: const EdgeInsets.symmetric(
        horizontal: Space.md + 2,
        vertical: Space.md,
      ),
      child: Row(
        children: [
          Expanded(
            child: Text(
              exam.courseName,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
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
