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
    // Lab summaries count each two-period session twice. Count sessions
    // from the history, which has one row each, or halve until it loads.
    final count = !record.islab()
        ? (standing.attended, standing.total)
        : history != null
        ? _sessionCount(history)
        : (standing.attended ~/ 2, standing.total ~/ 2);

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
                                      '${count.$1} of ${count.$2} attended',
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
                        // Plans from the summary counts, which match VTOP
                        // (in sessions for labs). Keyed so it resets when
                        // the lab history loads.
                        _Tab.planner => AttendancePlanner(
                          key: ValueKey(count),
                          attended: count.$1,
                          reported: standing.reported,
                          total: count.$2,
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

/// How the classes split (present / on duty / absent) as one bar, then a
/// month calendar with each class day marked by its status.
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
    if (statuses.isEmpty) return const SizedBox.shrink();
    int count(_Status s) => statuses.where((x) => x == s).length;
    final present = count(_Status.present);
    final onDuty = count(_Status.onDuty);
    final absent = count(_Status.absent);

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
    final note = [
      if (streak >= 3) '$streak in a row',
      if (lastMissed != null)
        'last missed ${DateFormat('d MMM').format(lastMissed)}'
      else
        'never missed',
    ].join(' · ');

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Surface(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _SplitBar(
                parts: [
                  (present, palette.success.base),
                  (onDuty, palette.accent),
                  (absent, palette.danger.base),
                ],
              ),
              const SizedBox(height: Space.md),
              Row(
                children: [
                  Expanded(
                    child: Text.rich(
                      TextSpan(
                        children: [
                          _countSpan(present, 'present', palette.success),
                          const TextSpan(text: '  ·  '),
                          _countSpan(onDuty, 'on duty', palette.accentTone),
                        ],
                      ),
                      style: typography.body.sm.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                  ),
                  Text.rich(
                    _countSpan(absent, 'absent', palette.danger),
                    style: typography.body.sm.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.xs),
              Text(
                note[0].toUpperCase() + note.substring(1),
                style: typography.body.xs.copyWith(
                  color: colors.mutedForeground,
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: Space.md),
        _MonthCalendar(records: data.records),
      ],
    );
  }

  static TextSpan _countSpan(int value, String label, Tone tone) => TextSpan(
    children: [
      TextSpan(
        text: '$value',
        style: TextStyle(
          fontWeight: FontWeight.w700,
          color: tone.onSubtle,
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
      ),
      TextSpan(text: ' $label'),
    ],
  );
}

/// A single bar split into proportional coloured parts; grows in on open.
class _SplitBar extends StatelessWidget {
  const _SplitBar({required this.parts});

  final List<(int, Color)> parts;

  @override
  Widget build(BuildContext context) {
    final shown = parts.where((p) => p.$1 > 0).toList();
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: 1),
      duration: Motion.slow * 2,
      curve: Curves.easeOutCubic,
      builder: (context, t, _) => ClipRRect(
        borderRadius: BorderRadius.circular(Radii.pill),
        child: SizedBox(
          height: 10,
          child: Align(
            alignment: Alignment.centerLeft,
            child: FractionallySizedBox(
              widthFactor: t,
              heightFactor: 1,
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (final (i, (value, color)) in shown.indexed) ...[
                    if (i > 0) const SizedBox(width: 2),
                    Expanded(
                      flex: value,
                      child: ColoredBox(color: color),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Month grid (Mon first) with each class day ringed in its status colour.
/// A day with several classes shows the worst one. Pages through the months
/// that have classes, opening on the latest.
class _MonthCalendar extends HookWidget {
  const _MonthCalendar({required this.records});

  final List<FullAttendanceRecord> records;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final palette = colors.app;

    // Worst status per day: absent > on duty > present.
    int rank(_Status s) => switch (s) {
      _Status.absent => 3,
      _Status.onDuty => 2,
      _Status.present => 1,
      _Status.other => 0,
    };
    final byDay = <DateTime, _Status>{};
    for (final r in records) {
      final d = _parseDate(r.date);
      if (d == null) continue;
      final s = _statusOf(r.status);
      final prev = byDay[d];
      if (prev == null || rank(s) > rank(prev)) byDay[d] = s;
    }
    if (byDay.isEmpty) return const SizedBox.shrink();

    final months = {
      for (final d in byDay.keys) DateTime(d.year, d.month),
    }.toList()..sort();
    final index = useState(months.length - 1);
    final i = index.value.clamp(0, months.length - 1);
    final month = months[i];
    final leading = month.weekday - 1; // Monday first
    final daysInMonth = DateTime(month.year, month.month + 1, 0).day;
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);

    Tone toneOf(_Status s) => switch (s) {
      _Status.present => palette.success,
      _Status.onDuty => palette.accentTone,
      _Status.absent => palette.danger,
      _Status.other => Tone(
        base: colors.mutedForeground,
        subtle: colors.secondary,
        onSubtle: colors.mutedForeground,
      ),
    };

    Widget arrow(IconData icon, bool enabled, int delta, String label) =>
        FButton.icon(
          variant: FButtonVariant.ghost,
          size: FButtonSizeVariant.sm,
          semanticsLabel: label,
          onPress: enabled ? () => index.value = i + delta : null,
          child: Icon(icon),
        );

    return Surface(
      padding: const EdgeInsets.fromLTRB(
        Space.md,
        Space.sm,
        Space.md,
        Space.md,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  DateFormat('MMMM y').format(month),
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
              ),
              arrow(FLucideIcons.chevronLeft, i > 0, -1, 'Previous month'),
              arrow(
                FLucideIcons.chevronRight,
                i < months.length - 1,
                1,
                'Next month',
              ),
            ],
          ),
          const SizedBox(height: Space.xs),
          Row(
            children: [
              for (final d in const ['M', 'T', 'W', 'T', 'F', 'S', 'S'])
                Expanded(
                  child: Center(
                    child: Text(
                      d,
                      style: typography.body.xs.copyWith(
                        fontSize: 10,
                        fontWeight: FontWeight.w600,
                        color: colors.mutedForeground,
                      ),
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(height: Space.xs),
          AnimatedSwitcher(
            duration: Motion.medium,
            child: Column(
              key: ValueKey(month),
              children: [
                for (var week = 0; week * 7 < leading + daysInMonth; week++)
                  Row(
                    children: [
                      for (var col = 0; col < 7; col++)
                        Expanded(
                          child: Builder(
                            builder: (context) {
                              final day = week * 7 + col - leading + 1;
                              if (day < 1 || day > daysInMonth) {
                                return const SizedBox(height: 36);
                              }
                              final date = DateTime(
                                month.year,
                                month.month,
                                day,
                              );
                              final status = byDay[date];
                              final tone = status == null
                                  ? null
                                  : toneOf(status);
                              return SizedBox(
                                height: 36,
                                child: Center(
                                  child: Container(
                                    width: 30,
                                    height: 30,
                                    alignment: Alignment.center,
                                    decoration: BoxDecoration(
                                      shape: BoxShape.circle,
                                      color: tone?.subtle,
                                      border: tone != null
                                          ? Border.all(
                                              color: tone.base,
                                              width: 1.5,
                                            )
                                          : date == today
                                          ? Border.all(color: colors.border)
                                          : null,
                                    ),
                                    child: Text(
                                      '$day',
                                      style: typography.body.xs.copyWith(
                                        fontWeight: tone != null
                                            ? FontWeight.w700
                                            : FontWeight.w400,
                                        color:
                                            tone?.onSubtle ??
                                            colors.mutedForeground,
                                        fontFeatures: const [
                                          FontFeature.tabularFigures(),
                                        ],
                                      ),
                                    ),
                                  ),
                                ),
                              );
                            },
                          ),
                        ),
                    ],
                  ),
              ],
            ),
          ),
        ],
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

/// (attended, total) sessions in [data]; on duty counts as attended.
(int, int) _sessionCount(FullAttendanceData data) {
  var attended = 0;
  var total = 0;
  for (final r in data.records) {
    switch (_statusOf(r.status)) {
      case _Status.present || _Status.onDuty:
        attended++;
        total++;
      case _Status.absent:
        total++;
      case _Status.other:
        break;
    }
  }
  return (attended, total);
}

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
