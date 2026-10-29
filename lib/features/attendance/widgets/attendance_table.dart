import 'package:vitapmate/observability/log.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/ui/extensions.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/attendance/logic/attendance_projection.dart';
import 'package:vitapmate/features/attendance/widgets/attendance_cal.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';

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
          Log.info('features.attendance', 'auto refresh failed: $e');
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

    final (code, name) = formatName(record.courseName);
    // In sessions for labs; exactly the history's count whenever it agrees.
    final standing = AttendanceStanding.sessions(record);
    final tone = attendanceTone(
      context,
      safe: standing.isSafe,
      atEdge: standing.canSkip == 0,
    );
    final history = dataAsync.value;
    final sync = history == null ? null : HistorySync.of(record, history);
    final count = (standing.attended, standing.total);
    final attendedLine = '${count.$1} of ${count.$2} attended';
    final hideSkips = !ref.watch(showSkipAdviceProvider);
    // Classes held since VTOP's last posted day, known only when the
    // history agrees with the summary (in sessions for labs).
    final timetable = ref.watch(timetableProvider).value;
    final calendar = ref.watch(semesterCalendarProvider);
    final classes = timetable == null || calendar == null
        ? null
        : AttendanceProjection.of(
            record: record,
            timetable: timetable,
            calendar: calendar,
            now: DateTime.now(),
            countedThrough: sync?.countedThrough,
          );
    final unposted =
        (record.islab() ? classes?.inSessions(record) : classes)?.unposted ?? 0;
    final strongMd = typography.body.md.copyWith(
      fontWeight: .w600,
      color: colors.foreground,
    );
    final mutedXs = typography.body.xs.copyWith(color: colors.mutedForeground);

    return ScreenRefresh(
      onRefresh: refresh,
      tasks: const ['vtop_full_attendance'],
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: colors.background,
          borderRadius: const .vertical(top: .circular(Radii.lg + 4)),
        ),
        child: Column(
          crossAxisAlignment: .stretch,
          children: [
            const SheetHandle(),
            Expanded(
              child: ListView(
                padding: .fromLTRB(
                  Space.lg,
                  Space.sm,
                  Space.lg,
                  Space.xl + MediaQuery.paddingOf(context).bottom,
                ),
                children: [
                  Row(
                    crossAxisAlignment: .start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: .start,
                          children: [
                            Text(
                              name.trim().isEmpty
                                  ? record.courseName
                                  : name.trim(),
                              style: typography.body.xl.copyWith(
                                fontWeight: .w700,
                                height: 1.2,
                                color: colors.foreground,
                              ),
                            ),
                            const SizedBox(height: Space.sm),
                            Row(
                              children: [
                                CourseKindBadge(isLab: record.islab()),
                                const SizedBox(width: Space.sm),
                                Text(code.trim(), style: mutedXs),
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
                        variant: .ghost,
                        semanticsLabel: 'Refresh attendance',
                        onPress: refreshing.value ? null : refresh,
                        child: refreshing.value
                            ? const FCircularProgress(size: .sm)
                            : const Icon(FLucideIcons.refreshCw),
                      ),
                    ],
                  ),
                  const SizedBox(height: Space.lg),
                  Surface(
                    child: Column(
                      crossAxisAlignment: .stretch,
                      children: [
                        Row(
                          crossAxisAlignment: .end,
                          children: [
                            CountUp(
                              value: standing.displayPercent,
                              suffix: '%',
                              style: typography.display.xl2.copyWith(
                                height: 1,
                                fontWeight: .w600,
                                letterSpacing: -0.5,
                                color: tone.base,
                              ),
                            ),
                            const SizedBox(width: Space.md),
                            Expanded(
                              child: Padding(
                                padding: const .only(bottom: 3),
                                child: Column(
                                  crossAxisAlignment: .start,
                                  children: [
                                    if (hideSkips)
                                      Text(attendedLine, style: strongMd)
                                    else ...[
                                      Text(standing.advice, style: strongMd),
                                      Text(attendedLine, style: mutedXs),
                                    ],
                                    // VTOP updates the history separately;
                                    // flag when it disagrees.
                                    if (sync != null && !sync.inSync)
                                      Row(
                                        children: [
                                          Dot(color: colors.app.warning.base),
                                          const SizedBox(width: Space.xs + 2),
                                          Expanded(
                                            child: Text(
                                              'History shows ${sync.attended} '
                                              'of ${sync.total} · pull to sync',
                                              maxLines: 1,
                                              overflow: .ellipsis,
                                              style: mutedXs,
                                            ),
                                          ),
                                        ],
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
                            style: mutedXs.copyWith(fontSize: 10),
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
                      alignment: .topCenter,
                      children: [...previous, ?current],
                    ),
                    child: KeyedSubtree(
                      key: ValueKey(tab.value),
                      child: switch (tab.value) {
                        .history => dataAsync.when(
                          skipLoadingOnRefresh: true,
                          data: (data) => _History(data: data),
                          error: (e, _) => EmptyState(
                            icon: FLucideIcons.cloudAlert,
                            title: "Couldn't load history",
                            message: commonErrorMessage(e),
                          ),
                          loading: () => const Padding(
                            padding: .only(top: Space.lg),
                            child: SkeletonList(count: 4, height: 72),
                          ),
                        ),
                        // Plans from the summary counts, which match VTOP
                        // (in sessions for labs). Keyed so it resets when
                        // a refresh changes them.
                        .planner => AttendancePlanner(
                          key: ValueKey(count),
                          attended: count.$1,
                          reported: standing.reported,
                          total: count.$2,
                          unposted: unposted,
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

/// [status]'s colours: green, accent, red, or muted for anything else.
Tone _toneOf(BuildContext context, ClassStatus status) {
  final colors = context.theme.colors;
  return switch (status) {
    .present => colors.app.success,
    .onDuty => colors.app.accentTone,
    .absent => colors.app.danger,
    .other => Tone(
      base: colors.mutedForeground,
      subtle: colors.secondary,
      onSubtle: colors.mutedForeground,
    ),
  };
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
    // Oldest first.
    final statuses = [
      for (final r in data.records.reversed) classStatusOf(r.status),
    ];
    if (statuses.isEmpty) return const SizedBox.shrink();
    int count(ClassStatus s) => statuses.where((x) => x == s).length;
    final present = count(.present);
    final onDuty = count(.onDuty);
    final absent = count(.absent);

    // Streak of attended (present or on duty) classes ending at the latest.
    final streak = statuses.reversed
        .takeWhile((s) => s == .present || s == .onDuty)
        .length;
    final lastMissed = data.records
        .where((r) => classStatusOf(r.status) == .absent)
        .map((r) => parseHistoryDate(r.date))
        .whereType<DateTime>()
        .firstOrNull;
    final note = [
      if (streak >= 3) '$streak in a row',
      if (lastMissed != null)
        'last missed ${DateFormat('d MMM').format(lastMissed)}'
      else
        'never missed',
    ].join(' · ');
    final muted = typography.body.sm.copyWith(color: colors.mutedForeground);

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        Surface(
          child: Column(
            crossAxisAlignment: .stretch,
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
                      style: muted,
                    ),
                  ),
                  Text.rich(
                    _countSpan(absent, 'absent', palette.danger),
                    style: muted,
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
        style: numStyle(fontWeight: .w700, color: tone.onSubtle),
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
        borderRadius: .circular(Radii.pill),
        child: SizedBox(
          height: 10,
          child: Align(
            alignment: .centerLeft,
            child: FractionallySizedBox(
              widthFactor: t,
              heightFactor: 1,
              child: Row(
                crossAxisAlignment: .stretch,
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

    // Worst status per day: absent > on duty > present.
    int rank(ClassStatus s) => switch (s) {
      .absent => 3,
      .onDuty => 2,
      .present => 1,
      .other => 0,
    };
    final byDay = <DateTime, ClassStatus>{};
    for (final r in records) {
      final d = parseHistoryDate(r.date);
      if (d == null) continue;
      final s = classStatusOf(r.status);
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

    Widget arrow(IconData icon, bool enabled, int delta, String label) =>
        FButton.icon(
          variant: .ghost,
          size: .sm,
          semanticsLabel: label,
          onPress: enabled ? () => index.value = i + delta : null,
          child: Icon(icon),
        );

    Widget day(int day) {
      if (day < 1 || day > daysInMonth) return const SizedBox(height: 36);
      final date = DateTime(month.year, month.month, day);
      final tone = switch (byDay[date]) {
        final status? => _toneOf(context, status),
        null => null,
      };
      return SizedBox(
        height: 36,
        child: Center(
          child: Container(
            width: 30,
            height: 30,
            alignment: .center,
            decoration: BoxDecoration(
              shape: .circle,
              color: tone?.subtle,
              border: tone != null
                  ? .all(color: tone.base, width: 1.5)
                  : date == today
                  ? .all(color: colors.border)
                  : null,
            ),
            child: Text(
              '$day',
              style: typography.body.xs.nums(
                fontWeight: tone != null ? .w700 : .w400,
                color: tone?.onSubtle ?? colors.mutedForeground,
              ),
            ),
          ),
        ),
      );
    }

    return Surface(
      padding: const .fromLTRB(Space.md, Space.sm, Space.md, Space.md),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  DateFormat('MMMM y').format(month),
                  style: typography.body.sm.copyWith(
                    fontWeight: .w600,
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
                        fontWeight: .w600,
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
                        Expanded(child: day(week * 7 + col - leading + 1)),
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
      final date = parseHistoryDate(record.date);
      final key = date == null ? 'Other' : DateFormat('MMMM y').format(date);
      groups.putIfAbsent(key, () => []).add(record);
    }

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        for (final MapEntry(key: month, value: records) in groups.entries) ...[
          SectionHeader(title: month, trailing: Text(_monthSummary(records))),
          Surface(
            padding: .zero,
            child: ClipRRect(
              borderRadius: .circular(Radii.lg - 1),
              child: Column(
                children: [
                  for (final (i, record) in records.indexed) ...[
                    if (i > 0)
                      Container(
                        height: 1,
                        margin: const .only(left: 60),
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
    final missed = records
        .where((r) => classStatusOf(r.status) == .absent)
        .length;
    return missed == 0
        ? '${records.length} classes'
        : '$missed missed of ${records.length}';
  }
}

class _HistoryRow extends StatelessWidget {
  const _HistoryRow({required this.record});

  final FullAttendanceRecord record;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final date = parseHistoryDate(record.date);
    final status = classStatusOf(record.status);
    final tone = _toneOf(context, status);
    final label = switch (status) {
      .present => 'Present',
      .onDuty => 'On duty',
      .absent => 'Absent',
      .other => record.status,
    };
    final absent = status == .absent;
    // "WED / 15:00-15:50" → "3:00 – 3:50 PM"
    final time = record.dayTime
        .split('/')
        .last
        .split('-')
        .map((t) => formatClock(t, context))
        .join(' – ');

    return Container(
      color: absent ? tone.subtle.withValues(alpha: 0.6) : null,
      padding: const .symmetric(horizontal: Space.md, vertical: Space.md - 2),
      child: Row(
        children: [
          SizedBox(
            width: 36,
            child: Column(
              children: [
                Text(
                  date == null ? '–' : DateFormat('d').format(date),
                  style: typography.body.lg.nums(
                    height: 1.1,
                    fontWeight: .w700,
                    color: colors.foreground,
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
              crossAxisAlignment: .start,
              children: [
                Row(
                  children: [
                    Dot(size: 8, color: tone.base),
                    const SizedBox(width: Space.sm),
                    Text(
                      label,
                      style: typography.body.sm.copyWith(
                        fontWeight: .w600,
                        color: absent ? tone.onSubtle : colors.foreground,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  '$time · Slot ${record.slot}',
                  style: typography.body.xs.nums(color: colors.mutedForeground),
                ),
                if (record.remark.trim().isNotEmpty)
                  Text(
                    record.remark.trim(),
                    maxLines: 2,
                    overflow: .ellipsis,
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
