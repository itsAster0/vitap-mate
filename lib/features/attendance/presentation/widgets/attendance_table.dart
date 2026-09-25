import 'dart:developer';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/attendance/presentation/providers/full_attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_cal.dart';
import 'package:vitapmate/features/timetable/presentation/utils/time_format.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum _Tab { history, planner }

/// Bottom sheet for one course: summary, class-by-class record, and a
/// planner. Everything scrolls as one page.
class AttendanceDetailSheet extends HookConsumerWidget {
  const AttendanceDetailSheet({super.key, required this.record});

  final AttendanceRecord record;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final provider = fullAttendanceProvider(record.courseType, record.courseId);
    final dataAsync = ref.watch(provider);
    final tab = useState(_Tab.history);
    final refreshing = useState(false);

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(provider.notifier).updateAttendance().catchError((e, st) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });
      return null;
    }, [record.courseType, record.courseId]);

    Future<void> refresh() async {
      refreshing.value = true;
      try {
        await ref.read(provider.notifier).updateAttendance();
      } catch (e) {
        if (context.mounted) disCommonToast(context, e);
      } finally {
        if (context.mounted) refreshing.value = false;
      }
    }

    final (code, name) = formateName(record.courseName);
    final standing = AttendanceStanding.of(record);
    final tone = attendanceTone(
      context,
      safe: standing.isSafe,
      atEdge: standing.canSkip == 0,
    );
    final history = dataAsync.value;

    return ScreenRefresh(
      onRefresh: refresh,
      tasks: const ['vtop_fullattendance'],
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colors.background,
          borderRadius: const BorderRadius.vertical(
            top: Radius.circular(Radii.lg + 4),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SheetHandle(),
            Expanded(
              child: ListView(
                padding: EdgeInsets.fromLTRB(
                  Space.lg,
                  Space.sm,
                  Space.lg,
                  Space.xl + MediaQuery.paddingOf(context).bottom,
                ),
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              name.trim().isEmpty
                                  ? record.courseName
                                  : name.trim(),
                              style: typography.body.xl.copyWith(
                                fontWeight: FontWeight.w700,
                                height: 1.2,
                                color: colors.foreground,
                              ),
                            ),
                            const SizedBox(height: Space.sm),
                            Row(
                              children: [
                                CourseKindBadge(isLab: record.islab()),
                                const SizedBox(width: Space.sm),
                                Text(
                                  code.trim(),
                                  style: typography.body.xs.copyWith(
                                    color: colors.mutedForeground,
                                  ),
                                ),
                              ],
                            ),
                            if (record.facultyDetail.trim().isNotEmpty) ...[
                              const SizedBox(height: Space.xs + 2),
                              FacultyLine(
                                name: record.facultyDetail
                                    .split(' - ')
                                    .first
                                    .trim(),
                              ),
                            ],
                          ],
                        ),
                      ),
                      FButton.icon(
                        variant: FButtonVariant.ghost,
                        semanticsLabel: 'Refresh attendance',
                        onPress: refreshing.value ? null : refresh,
                        child: refreshing.value
                            ? const FCircularProgress(
                                size: FCircularProgressSizeVariant.sm,
                              )
                            : const Icon(FLucideIcons.refreshCw),
                      ),
                    ],
                  ),
                  const SizedBox(height: Space.lg),
                  Surface(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.end,
                          children: [
                            CountUp(
                              value: standing.displayPercent,
                              suffix: '%',
                              style: typography.display.xl2.copyWith(
                                height: 1,
                                fontWeight: FontWeight.w600,
                                letterSpacing: -0.5,
                                color: tone.base,
                              ),
                            ),
                            const SizedBox(width: Space.md),
                            Expanded(
                              child: Padding(
                                padding: const EdgeInsets.only(bottom: 3),
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      standing.advice,
                                      style: typography.body.md.copyWith(
                                        fontWeight: FontWeight.w600,
                                        color: colors.foreground,
                                      ),
                                    ),
                                    Text(
                                      '${standing.attended} of ${standing.total} attended',
                                      style: typography.body.xs.copyWith(
                                        color: colors.mutedForeground,
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: Space.md),
                        SkipMeter(percent: standing.displayPercent, tone: tone),
                        const SizedBox(height: Space.xs),
                        Align(
                          alignment: const Alignment(0.5, 0),
                          child: Text(
                            '75% required',
                            style: typography.body.xs.copyWith(
                              fontSize: 10,
                              color: colors.mutedForeground,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: Space.md),
                  AnimatedSwitcher(
                    duration: Motion.medium,
                    child: history == null
                        ? const Column(
                            key: ValueKey('loading'),
                            children: [
                              Skeleton(height: 64, radius: Radii.lg),
                              SizedBox(height: Space.md),
                              Skeleton(height: 90, radius: Radii.lg),
                            ],
                          )
                        : _Insights(key: const ValueKey('data'), data: history),
                  ),
                  const SizedBox(height: Space.lg),
                  Segmented<_Tab>(
                    value: tab.value,
                    onChanged: (value) => tab.value = value,
                    segments: const [
                      (_Tab.history, 'History'),
                      (_Tab.planner, 'Planner'),
                    ],
                  ),
                  AnimatedSwitcher(
                    duration: Motion.medium,
                    switchInCurve: const Interval(
                      0.3,
                      1,
                      curve: Curves.easeOut,
                    ),
                    switchOutCurve: const Interval(
                      0.7,
                      1,
                      curve: Curves.easeIn,
                    ),
                    layoutBuilder: (current, previous) => Stack(
                      alignment: Alignment.topCenter,
                      children: [...previous, ?current],
                    ),
                    child: KeyedSubtree(
                      key: ValueKey(tab.value),
                      child: switch (tab.value) {
                        _Tab.history => dataAsync.when(
                          skipLoadingOnRefresh: true,
                          data: (data) => _History(data: data),
                          error: (e, _) => EmptyState(
                            icon: FLucideIcons.cloudAlert,
                            title: "Couldn't load history",
                            message: commonErrorMessage(e),
                          ),
                          loading: () => const Padding(
                            padding: EdgeInsets.only(top: Space.lg),
                            child: SkeletonList(count: 4, height: 72),
                          ),
                        ),
                        // Plans from the summary counts, which match VTOP.
                        _Tab.planner => AttendancePlanner(
                          attended: standing.attended,
                          total: standing.total,
                        ),
                      },
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Present / on-duty / absent counts and a square per class, oldest first.
class _Insights extends StatelessWidget {
  const _Insights({super.key, required this.data});

  final FullAttendanceData data;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final palette = colors.app;
    final statuses = [
      for (final r in data.records.reversed) _statusOf(r.status),
    ];
    int count(_Status s) => statuses.where((x) => x == s).length;

    // Streak of attended (present or on duty) classes ending at the latest.
    var streak = 0;
    for (final s in statuses.reversed) {
      if (s == _Status.present || s == _Status.onDuty) {
        streak++;
      } else {
        break;
      }
    }
    final lastMissed = data.records
        .where((r) => _statusOf(r.status) == _Status.absent)
        .map((r) => _parseDate(r.date))
        .whereType<DateTime>()
        .firstOrNull;

    final notes = [
      if (streak >= 3) '$streak in a row',
      if (lastMissed != null)
        'Last missed ${DateFormat('d MMM').format(lastMissed)}'
      else if (statuses.isNotEmpty)
        'Never missed',
    ];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: _StatTile(
                label: 'Present',
                value: count(_Status.present),
                tone: palette.success,
              ),
            ),
            const SizedBox(width: Space.sm),
            Expanded(
              child: _StatTile(
                label: 'On duty',
                value: count(_Status.onDuty),
                tone: palette.accentTone,
              ),
            ),
            const SizedBox(width: Space.sm),
            Expanded(
              child: _StatTile(
                label: 'Absent',
                value: count(_Status.absent),
                tone: palette.danger,
              ),
            ),
          ],
        ),
        if (statuses.isNotEmpty) ...[
          const SizedBox(height: Space.md),
          Surface(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      'CLASS BY CLASS',
                      style: typography.body.xs.copyWith(
                        fontWeight: FontWeight.w600,
                        letterSpacing: 0.5,
                        color: colors.mutedForeground,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: Space.md),
                Wrap(
                  spacing: 4,
                  runSpacing: 4,
                  children: [
                    for (final (i, s) in statuses.indexed)
                      _ClassSquare(status: s, index: i),
                  ],
                ),
                const SizedBox(height: Space.sm),
                Text(
                  'Oldest → latest',
                  style: typography.body.xs.copyWith(
                    fontSize: 10,
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
        ],
      ],
    );
  }
}

class _StatTile extends StatelessWidget {
  const _StatTile({
    required this.label,
    required this.value,
    required this.tone,
  });

  final String label;
  final int value;
  final Tone tone;

  @override
  Widget build(BuildContext context) {
    final typography = context.theme.typography;
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.md,
        vertical: Space.md - 2,
      ),
      decoration: BoxDecoration(
        color: tone.subtle,
        borderRadius: BorderRadius.circular(Radii.md),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CountUp(
            value: value.toDouble(),
            style: typography.body.xl.copyWith(
              height: 1.1,
              fontWeight: FontWeight.w600,
              color: tone.onSubtle,
            ),
          ),
          Text(
            label,
            style: typography.body.xs.copyWith(
              fontWeight: FontWeight.w600,
              color: tone.onSubtle,
            ),
          ),
        ],
      ),
    );
  }
}

class _ClassSquare extends StatelessWidget {
  const _ClassSquare({required this.status, required this.index});

  final _Status status;
  final int index;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final palette = colors.app;
    final color = switch (status) {
      _Status.present => palette.success.base,
      _Status.onDuty => palette.accent,
      _Status.absent => palette.danger.base,
      _Status.other => colors.mutedForeground,
    };
    // Squares pop in one after another.
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: 1),
      duration: Duration(milliseconds: 200 + 18 * index),
      curve: Curves.easeOutBack,
      builder: (context, t, child) =>
          Transform.scale(scale: t.clamp(0.0, 1.2), child: child),
      child: Container(
        width: 14,
        height: 14,
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(3),
        ),
      ),
    );
  }
}

class _History extends StatelessWidget {
  const _History({required this.data});

  final FullAttendanceData data;

  @override
  Widget build(BuildContext context) {
    if (data.records.isEmpty) {
      return const EmptyState(
        icon: FLucideIcons.calendarX,
        title: 'No classes recorded yet',
      );
    }

    // VTOP lists newest first; group consecutive records by month.
    final groups = <String, List<FullAttendanceRecord>>{};
    for (final record in data.records) {
      final date = _parseDate(record.date);
      final key = date == null ? 'Other' : DateFormat('MMMM y').format(date);
      groups.putIfAbsent(key, () => []).add(record);
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final entry in groups.entries) ...[
          SectionHeader(
            title: entry.key,
            trailing: Text(_monthSummary(entry.value)),
          ),
          Surface(
            padding: EdgeInsets.zero,
            child: ClipRRect(
              borderRadius: BorderRadius.circular(Radii.lg - 1),
              child: Column(
                children: [
                  for (final (i, record) in entry.value.indexed) ...[
                    if (i > 0)
                      Container(
                        height: 1,
                        margin: const EdgeInsets.only(left: 60),
                        color: context.theme.colors.border,
                      ),
                    _HistoryRow(record: record),
                  ],
                ],
              ),
            ),
          ),
        ],
        DataUpdatedFooter(updateTime: data.updateTime.toInt()),
      ],
    );
  }

  String _monthSummary(List<FullAttendanceRecord> records) {
    final missed = records.where((r) => _statusOf(r.status) == _Status.absent);
    return missed.isEmpty
        ? '${records.length} classes'
        : '${missed.length} missed of ${records.length}';
  }
}

enum _Status { present, onDuty, absent, other }

_Status _statusOf(String raw) {
  final s = raw.toLowerCase().replaceAll(' ', '');
  if (s == 'present') return _Status.present;
  if (s == 'onduty') return _Status.onDuty;
  if (s == 'absent') return _Status.absent;
  return _Status.other;
}

DateTime? _parseDate(String value) {
  try {
    return DateFormat('dd-MM-yyyy').parseStrict(value);
  } catch (_) {
    return null;
  }
}

class _HistoryRow extends StatelessWidget {
  const _HistoryRow({required this.record});

  final FullAttendanceRecord record;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final palette = colors.app;
    final date = _parseDate(record.date);
    final status = _statusOf(record.status);
    final (tone, label) = switch (status) {
      _Status.present => (palette.success, 'Present'),
      _Status.onDuty => (palette.accentTone, 'On duty'),
      _Status.absent => (palette.danger, 'Absent'),
      _Status.other => (
        Tone(
          base: colors.mutedForeground,
          subtle: colors.secondary,
          onSubtle: colors.mutedForeground,
        ),
        record.status,
      ),
    };
    final absent = status == _Status.absent;
    // "WED / 15:00-15:50" → "3:00 – 3:50 PM"
    final time = record.dayTime
        .split('/')
        .last
        .split('-')
        .map((t) => formatClock(t, context))
        .join(' – ');

    return Container(
      color: absent ? tone.subtle.withValues(alpha: 0.6) : null,
      padding: const EdgeInsets.symmetric(
        horizontal: Space.md,
        vertical: Space.md - 2,
      ),
      child: Row(
        children: [
          SizedBox(
            width: 36,
            child: Column(
              children: [
                Text(
                  date == null ? '–' : DateFormat('d').format(date),
                  style: typography.body.lg.copyWith(
                    height: 1.1,
                    fontWeight: FontWeight.w700,
                    color: colors.foreground,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
                Text(
                  date == null
                      ? ''
                      : DateFormat('EEE').format(date).toUpperCase(),
                  style: typography.body.xs.copyWith(
                    fontSize: 10,
                    letterSpacing: 0.4,
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(
                        color: tone.base,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: Space.sm),
                    Text(
                      label,
                      style: typography.body.sm.copyWith(
                        fontWeight: FontWeight.w600,
                        color: absent ? tone.onSubtle : colors.foreground,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  '$time · Slot ${record.slot}',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
                if (record.remark.trim().isNotEmpty)
                  Text(
                    record.remark.trim(),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: typography.body.xs.copyWith(
                      color: absent ? tone.onSubtle : colors.mutedForeground,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
