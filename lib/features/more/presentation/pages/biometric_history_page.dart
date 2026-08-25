import 'dart:developer' show log;

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/features/more/presentation/providers/biometric_history_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
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

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final data = ref.watch(biometricHistoryProvider(_vtopDate(_selectedDate)));

    return RefreshIndicator(
      onRefresh: _refresh,
      displacement: 80,
      backgroundColor: colors.primary,
      color: colors.primaryForeground,
      child: CustomScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        slivers: [
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(0, 8, 0, 10),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  FTileGroup(
                    children: [
                      FTile(
                        prefix: const Icon(FLucideIcons.calendarDays),
                        title: const Text('Selected date'),
                        subtitle: Text(
                          DateFormat('EEEE, d MMMM yyyy').format(_selectedDate),
                        ),
                        suffix: const Icon(FLucideIcons.chevronDown),
                        onPress: _pickDate,
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  SizedBox(
                    height: 38,
                    child: ListView.separated(
                      scrollDirection: Axis.horizontal,
                      itemCount: 7,
                      separatorBuilder: (_, _) => const SizedBox(width: 8),
                      itemBuilder: (context, index) {
                        final date = DateTime.now().subtract(
                          Duration(days: index),
                        );
                        final selected =
                            _vtopDate(date) == _vtopDate(_selectedDate);
                        return FButton(
                          variant: selected
                              ? FButtonVariant.primary
                              : FButtonVariant.outline,
                          size: .sm,
                          mainAxisSize: MainAxisSize.min,
                          prefix: selected
                              ? const Icon(FLucideIcons.check)
                              : null,
                          child: Text(
                            index == 0
                                ? 'Today'
                                : DateFormat('EEE, d').format(date),
                          ),
                          onPress: () async {
                            setState(() => _selectedDate = date);
                            if (await isAutoRefreshEnabled(ref)) await _load();
                          },
                        );
                      },
                    ),
                  ),
                ],
              ),
            ),
          ),
          data.when(
            loading: () => const SliverFillRemaining(
              child: Center(child: SizedBox(width: 180, child: FProgress())),
            ),
            error: (error, _) => SliverFillRemaining(
              child: _MessageState(
                icon: FLucideIcons.cloudOff,
                title: 'Could not load biometric history',
                message: commonErrorMessage(error),
                action: _refresh,
              ),
            ),
            data: (data) => data.records.isEmpty
                ? SliverFillRemaining(
                    child: Column(
                      children: [
                        const Expanded(
                          child: _MessageState(
                            icon: FLucideIcons.fingerprint,
                            title: 'No punches found',
                            message:
                                'There are no biometric or face logs for this date.',
                          ),
                        ),
                        DataUpdatedFooter(
                          updateTime: data.updateTime.toInt(),
                          padding: const EdgeInsets.only(bottom: 16),
                        ),
                      ],
                    ),
                  )
                : SliverPadding(
                    padding: const EdgeInsets.fromLTRB(0, 4, 0, 20),
                    sliver: SliverList.list(
                      children: [
                        _Summary(records: data.records),
                        const SizedBox(height: 10),
                        _BiometricTable(
                          records: data.records,
                          darkMode: darkMode,
                        ),
                        DataUpdatedFooter(
                          updateTime: data.updateTime.toInt(),
                          padding: const EdgeInsets.only(top: 12),
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

class _Summary extends StatelessWidget {
  const _Summary({required this.records});
  final List<BiometricRecord> records;

  @override
  Widget build(BuildContext context) {
    final inside = records
        .where((e) => e.venue.toUpperCase().contains('-IN-'))
        .length;
    final outside = records
        .where((e) => e.venue.toUpperCase().contains('-OUT-'))
        .length;
    return Row(
      children: [
        _Count(label: 'Total', value: records.length),
        const SizedBox(width: 8),
        _Count(label: 'Entries', value: inside),
        const SizedBox(width: 8),
        _Count(label: 'Exits', value: outside),
      ],
    );
  }
}

class _Count extends StatelessWidget {
  const _Count({required this.label, required this.value});
  final String label;
  final int value;

  @override
  Widget build(BuildContext context) => Expanded(
    child: Container(
      padding: const EdgeInsets.symmetric(vertical: 12),
      decoration: BoxDecoration(
        color: context.theme.colors.primaryForeground,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Column(
        children: [
          Text(
            '$value',
            style: context.theme.typography.body.xl.copyWith(
              fontSize: 20,
              fontWeight: FontWeight.w800,
            ),
          ),
          Text(
            label,
            style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
          ),
        ],
      ),
    ),
  );
}

class _BiometricTable extends StatelessWidget {
  const _BiometricTable({required this.records, required this.darkMode});

  final List<BiometricRecord> records;
  final bool darkMode;

  String _type(BiometricRecord record) {
    final venue = record.venue.toUpperCase();
    if (venue.contains('-OUT-')) return 'Exit';
    if (venue.contains('-IN-')) return 'Entry';
    return 'Punch';
  }

  String _time(String value) =>
      value.split(':').map((part) => part.padLeft(2, '0')).join(':');

  @override
  Widget build(BuildContext context) {
    final textColor = darkMode
        ? context.theme.colors.primary
        : MoreColors.secondaryText;

    return Container(
      decoration: BoxDecoration(
        color: darkMode
            ? context.theme.colors.primaryForeground
            : MoreColors.tableBackground,
        borderRadius: BorderRadius.circular(12),
        boxShadow: const [
          BoxShadow(
            color: MoreColors.cardShadowSecondary,
            blurRadius: 8,
            offset: Offset(0, 2),
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(12),
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: DataTable(
            dividerThickness: darkMode ? 0 : 1,
            headingRowColor: WidgetStatePropertyAll(
              darkMode
                  ? context.theme.colors.primaryForeground
                  : MoreColors.tableHeaderBackground,
            ),
            headingRowHeight: 56,
            dataRowMinHeight: 48,
            dataRowMaxHeight: 64,
            columnSpacing: 24,
            horizontalMargin: 16,
            columns: [
              _column('#', textColor, numeric: true),
              _column('Date', textColor),
              _column('Time', textColor),
              _column('Type', textColor),
              _column('Venue', textColor),
            ],
            rows: records.asMap().entries.map((entry) {
              final record = entry.value;
              final type = _type(record);
              final isEven = entry.key.isEven;
              return DataRow(
                color: WidgetStatePropertyAll(
                  darkMode || isEven
                      ? Colors.transparent
                      : MoreColors.tableRowAlternate,
                ),
                cells: [
                  _cell(record.serial, textColor, numeric: true),
                  _cell(record.punchDate, textColor),
                  _cell(_time(record.punchTime), textColor, numeric: true),
                  DataCell(_TypeBadge(type: type)),
                  _cell(record.venue, textColor),
                ],
              );
            }).toList(),
          ),
        ),
      ),
    );
  }

  DataColumn _column(String label, Color color, {bool numeric = false}) {
    return DataColumn(
      numeric: numeric,
      label: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 14,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }

  DataCell _cell(String value, Color color, {bool numeric = false}) {
    return DataCell(
      Text(
        value,
        style: TextStyle(
          color: color,
          fontSize: 13,
          fontWeight: numeric ? FontWeight.w600 : FontWeight.w400,
          fontFeatures: numeric ? const [FontFeature.tabularFigures()] : null,
        ),
      ),
    );
  }
}

class _TypeBadge extends StatelessWidget {
  const _TypeBadge({required this.type});

  final String type;

  @override
  Widget build(BuildContext context) {
    final isExit = type == 'Exit';
    final isEntry = type == 'Entry';
    final color = isExit
        ? MoreColors.errorText
        : isEntry
        ? MoreColors.successText
        : MoreColors.infoText;
    final background = isExit
        ? MoreColors.errorBackground
        : isEntry
        ? MoreColors.successBackground
        : MoreColors.infoBackground;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: color.withValues(alpha: 0.35)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            isExit
                ? FLucideIcons.logOut
                : isEntry
                ? FLucideIcons.logIn
                : FLucideIcons.fingerprint,
            size: 12,
            color: color,
          ),
          const SizedBox(width: 4),
          Text(
            type,
            style: TextStyle(
              color: color,
              fontSize: 12,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}

class _MessageState extends StatelessWidget {
  const _MessageState({
    required this.icon,
    required this.title,
    required this.message,
    this.action,
  });
  final IconData icon;
  final String title;
  final String message;
  final Future<void> Function()? action;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(28),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 52, color: context.theme.colors.mutedForeground),
          const SizedBox(height: 14),
          Text(
            title,
            style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w800),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 6),
          Text(
            message,
            textAlign: TextAlign.center,
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
          if (action != null) ...[
            const SizedBox(height: 16),
            FButton(
              onPress: action,
              prefix: const Icon(FLucideIcons.refreshCw),
              child: const Text('Try again'),
            ),
          ],
        ],
      ),
    ),
  );
}
