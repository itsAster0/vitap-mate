import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/features/more/presentation/providers/biometric_history_provider.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class BiometricHistoryPage extends ConsumerStatefulWidget {
  const BiometricHistoryPage({super.key});

  @override
  ConsumerState<BiometricHistoryPage> createState() =>
      _BiometricHistoryPageState();
}

class _BiometricHistoryPageState extends ConsumerState<BiometricHistoryPage> {
  late DateTime _selectedDate;

  @override
  void initState() {
    super.initState();
    _selectedDate = DateTime.now();
    _scheduleAutoRefresh();
  }

  String _vtopDate(DateTime date) => DateFormat('dd/MM/yyyy').format(date);

  Future<void> _load() async {
    await ref
        .read(biometricHistoryProvider(_vtopDate(_selectedDate)).notifier)
        .refresh();
  }

  Future<void> _refresh() async {
    try {
      await _load();
    } catch (e) {
      log('$e');
      if (mounted) disCommonToast(context, e);
    }
  }

  void _scheduleAutoRefresh() {
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      if (!mounted || !await isAutoRefreshEnabled(ref)) return;
      _load().catchError((e, st) {
        log('auto refresh failed: $e', stackTrace: st);
      });
    });
  }

  Future<void> _pickDate() async {
    DateTime candidate = _selectedDate;
    final picked = await showFDialog<DateTime>(
      context: context,
      useSafeArea: true,
      builder: (dialogContext, _, _) => Center(
        child: SizedBox(
          height: 430,
          child: FCalendar.grid(
            control: FGridCalendarControl(
              start: DateTime(2020),
              end: DateTime.now().add(const Duration(days: 1)),
              initial: _selectedDate,
            ),
            selectionControl: FDateSelectionControl.liftedSingle(
              value: candidate,
              onChange: (date) {
                if (date != null) candidate = date;
              },
            ),
            onDayPress: (date) {
              Navigator.of(dialogContext).pop(date);
            },
          ),
        ),
      ),
    );
    if (picked == null || picked == _selectedDate) return;
    setState(() => _selectedDate = picked);
    if (await isAutoRefreshEnabled(ref)) await _load();
  }

  Future<void> _select(DateTime date) async {
    setState(() => _selectedDate = date);
    if (await isAutoRefreshEnabled(ref)) await _load();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final data = ref.watch(biometricHistoryProvider(_vtopDate(_selectedDate)));
    final now = DateTime.now();
    final isToday = _vtopDate(_selectedDate) == _vtopDate(now);

    return ScreenRefresh(
      onRefresh: _refresh,
      tasks: const ['vtop_biometric_history'],
      child: RefreshIndicator(
        onRefresh: _refresh,
        backgroundColor: colors.background,
        color: colors.foreground,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(
            Space.sm,
            Space.sm,
            Space.sm,
            Space.lg,
          ),
          children: [
            // Last week as chips, plus a calendar for older dates.
            SizedBox(
              height: 36,
              child: ListView(
                scrollDirection: Axis.horizontal,
                children: [
                  for (var i = 0; i < 7; i++)
                    Builder(
                      builder: (context) {
                        final date = now.subtract(Duration(days: i));
                        final selected =
                            _vtopDate(date) == _vtopDate(_selectedDate);
                        return Padding(
                          padding: const EdgeInsets.only(right: Space.sm),
                          child: _DateChip(
                            label: i == 0
                                ? 'Today'
                                : i == 1
                                ? 'Yesterday'
                                : DateFormat('EEE d').format(date),
                            selected: selected,
                            onPress: () => _select(date),
                          ),
                        );
                      },
                    ),
                  _DateChip(
                    label: 'Pick date',
                    icon: FLucideIcons.calendarDays,
                    selected: now.difference(_selectedDate).inDays >= 7,
                    onPress: _pickDate,
                  ),
                ],
              ),
            ),
            const SizedBox(height: Space.md),
            AnimatedSwitcher(
              duration: Motion.medium,
              child: data.when(
                skipLoadingOnRefresh: true,
                loading: () => const Column(
                  key: ValueKey('loading'),
                  children: [
                    Skeleton(height: 72, radius: Radii.lg),
                    SizedBox(height: Space.md),
                    SkeletonList(count: 3, height: 64),
                  ],
                ),
                error: (error, _) => EmptyState(
                  key: const ValueKey('error'),
                  icon: FLucideIcons.cloudOff,
                  title: "Couldn't load biometric history",
                  message: commonErrorMessage(error),
                  action: FButton(
                    variant: FButtonVariant.outline,
                    mainAxisSize: MainAxisSize.min,
                    onPress: _refresh,
                    child: const Text('Try again'),
                  ),
                ),
                data: (data) {
                  final records = [...data.records]
                    ..sort(
                      (a, b) => _secondsOf(
                        b.punchTime,
                      ).compareTo(_secondsOf(a.punchTime)),
                    );
                  return Column(
                    key: ValueKey('data_${_vtopDate(_selectedDate)}'),
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (records.isEmpty)
                        EmptyState(
                          icon: FLucideIcons.scanFace,
                          title: 'No punches',
                          message:
                              'No face or biometric logs on ${DateFormat('EEE d MMM').format(_selectedDate)}.',
                        )
                      else ...[
                        _Status(latest: records.first, isToday: isToday),
                        SectionHeader(
                          title: DateFormat(
                            'EEEE, d MMMM',
                          ).format(_selectedDate),
                          trailing: Text(
                            '${records.where((r) => _kindOf(r) == _Kind.entry).length} in · '
                            '${records.where((r) => _kindOf(r) == _Kind.exit).length} out',
                          ),
                        ),
                        Surface(
                          padding: const EdgeInsets.symmetric(
                            horizontal: Space.md + 2,
                            vertical: Space.sm,
                          ),
                          child: Column(
                            children: [
                              for (final (i, r) in records.indexed)
                                _PunchRow(
                                  record: r,
                                  isFirst: i == 0,
                                  isLast: i == records.length - 1,
                                ),
                            ],
                          ),
                        ),
                      ],
                      DataUpdatedFooter(updateTime: data.updateTime.toInt()),
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

enum _Kind { entry, exit, other }

_Kind _kindOf(BiometricRecord r) {
  final venue = r.venue.toUpperCase();
  if (venue.contains('-OUT-')) return _Kind.exit;
  if (venue.contains('-IN-')) return _Kind.entry;
  return _Kind.other;
}

/// "MH2-FACE-IN-5" → "MH2".
String _placeOf(BiometricRecord r) => r.venue.split('-').first.trim();

/// "13:16:05" → "1:16 PM", matching the 12-hour times used elsewhere.
/// VTOP sometimes drops the minute's leading zero ("13:1"), so parse parts.
String _hm(String time) {
  final parts = time.trim().split(':');
  if (parts.length < 2) return time.trim();
  final h = int.tryParse(parts[0]);
  final m = int.tryParse(parts[1]);
  if (h == null || m == null) return time.trim();
  return DateFormat('h:mm a').format(DateTime(2000, 1, 1, h, m));
}

/// Seconds since midnight, for ordering punches.
int _secondsOf(String time) {
  final p = time.trim().split(':').map((e) => int.tryParse(e) ?? 0).toList();
  return (p.isNotEmpty ? p[0] : 0) * 3600 +
      (p.length > 1 ? p[1] : 0) * 60 +
      (p.length > 2 ? p[2] : 0);
}

class _Status extends StatelessWidget {
  const _Status({required this.latest, required this.isToday});

  final BiometricRecord latest;
  final bool isToday;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final kind = _kindOf(latest);
    final place = _placeOf(latest);
    final tone = switch (kind) {
      _Kind.entry => colors.app.success,
      _Kind.exit => colors.app.warning,
      _Kind.other => colors.app.accentTone,
    };
    final (title, detail) = isToday
        ? switch (kind) {
            _Kind.entry => ('Inside $place', 'since ${_hm(latest.punchTime)}'),
            _Kind.exit => ('Out of $place', 'since ${_hm(latest.punchTime)}'),
            _Kind.other => ('Last seen at $place', _hm(latest.punchTime)),
          }
        : (
            'Last punch that day',
            '${kind == _Kind.exit ? 'Exit' : 'Entry'} at $place, ${_hm(latest.punchTime)}',
          );

    return Surface(
      child: Row(
        children: [
          Container(
            width: 10,
            height: 10,
            decoration: BoxDecoration(
              color: tone.base,
              shape: BoxShape.circle,
              boxShadow: [
                BoxShadow(
                  color: tone.base.withValues(alpha: 0.4),
                  blurRadius: 6,
                  spreadRadius: 1,
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: typography.body.md.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
                Text(
                  detail,
                  style: typography.body.sm.copyWith(
                    color: colors.mutedForeground,
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

class _PunchRow extends StatelessWidget {
  const _PunchRow({
    required this.record,
    required this.isFirst,
    required this.isLast,
  });

  final BiometricRecord record;
  final bool isFirst;
  final bool isLast;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final kind = _kindOf(record);
    final (tone, label, icon) = switch (kind) {
      _Kind.entry => (colors.app.success, 'ENTRY', FLucideIcons.logIn),
      _Kind.exit => (colors.app.warning, 'EXIT', FLucideIcons.logOut),
      _Kind.other => (colors.app.accentTone, 'PUNCH', FLucideIcons.scanFace),
    };

    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            width: 64,
            child: Padding(
              padding: const EdgeInsets.only(top: Space.md),
              child: Text(
                _hm(record.punchTime),
                style: typography.body.sm.copyWith(
                  fontWeight: FontWeight.w500,
                  color: colors.foreground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ),
          ),
          SizedBox(
            width: 24,
            child: Stack(
              alignment: Alignment.topCenter,
              children: [
                Positioned(
                  top: isFirst ? 18 : 0,
                  bottom: isLast ? null : 0,
                  height: isLast ? 18 : null,
                  child: Container(width: 1.5, color: colors.border),
                ),
                Positioned(
                  top: 14,
                  child: Container(
                    width: 9,
                    height: 9,
                    decoration: BoxDecoration(
                      color: tone.base,
                      shape: BoxShape.circle,
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.sm),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: Space.sm + 2),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  ToneBadge(label: label, tone: tone, icon: icon),
                  const SizedBox(height: Space.xs),
                  Text(
                    record.venue.trim(),
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
    );
  }
}

class _DateChip extends StatelessWidget {
  const _DateChip({
    required this.label,
    required this.selected,
    required this.onPress,
    this.icon,
  });

  final String label;
  final bool selected;
  final VoidCallback onPress;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final fg = selected ? colors.primaryForeground : colors.foreground;
    return PressScale(
      scale: 0.95,
      semanticsLabel: label,
      onPress: onPress,
      child: AnimatedContainer(
        duration: Motion.medium,
        padding: const EdgeInsets.symmetric(horizontal: Space.md),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: selected ? colors.primary : colors.card,
          borderRadius: BorderRadius.circular(Radii.pill),
          border: Border.all(color: selected ? colors.primary : colors.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (icon != null) ...[
              Icon(icon, size: 14, color: fg),
              const SizedBox(width: Space.xs + 2),
            ],
            Text(
              label,
              style: context.theme.typography.body.sm.copyWith(
                fontWeight: FontWeight.w500,
                color: fg,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
