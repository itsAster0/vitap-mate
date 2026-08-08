import 'dart:developer';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class BiometricHistoryPage extends ConsumerStatefulWidget {
  const BiometricHistoryPage({super.key});

  @override
  ConsumerState<BiometricHistoryPage> createState() =>
      _BiometricHistoryPageState();
}

class _BiometricHistoryPageState extends ConsumerState<BiometricHistoryPage> {
  late DateTime _selectedDate;
  AsyncValue<BiometricData> _data = const AsyncLoading();

  @override
  void initState() {
    super.initState();
    _selectedDate = DateTime.now();
    Future.microtask(_load);
  }

  String _vtopDate(DateTime date) => DateFormat('dd/MM/yyyy').format(date);

  Future<void> _load() async {
    if (mounted) setState(() => _data = const AsyncLoading());
    try {
      await ref.read(vClientProvider.notifier).ensureLogin();
      final result = await fetchBiometricHistory(
        client: await ref.read(vClientProvider.future),
        date: _vtopDate(_selectedDate),
      );
      if (mounted) setState(() => _data = AsyncData(result));
    } catch (error, stackTrace) {
      log(
        'Unable to load biometric history',
        error: error,
        stackTrace: stackTrace,
      );
      if (mounted) setState(() => _data = AsyncError(error, stackTrace));
    }
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
    await _load();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return FScaffold(
      childPad: false,
      child: CustomScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        slivers: [
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 18, 16, 10),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Biometric history',
                    style: context.theme.typography.display.xl3.copyWith(
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Face and biometric punches recorded by VTOP',
                    style: TextStyle(color: colors.mutedForeground),
                  ),
                  const SizedBox(height: 16),
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
                            await _load();
                          },
                        );
                      },
                    ),
                  ),
                ],
              ),
            ),
          ),
          _data.when(
            loading: () => const SliverFillRemaining(
              child: Center(child: SizedBox(width: 180, child: FProgress())),
            ),
            error: (error, _) => SliverFillRemaining(
              child: _MessageState(
                icon: FLucideIcons.cloudOff,
                title: 'Could not load biometric history',
                message: commonErrorMessage(error),
                action: _load,
              ),
            ),
            data: (data) => data.records.isEmpty
                ? const SliverFillRemaining(
                    child: _MessageState(
                      icon: FLucideIcons.fingerprint,
                      title: 'No punches found',
                      message:
                          'There are no biometric or face logs for this date.',
                    ),
                  )
                : SliverPadding(
                    padding: const EdgeInsets.fromLTRB(16, 4, 16, 28),
                    sliver: SliverList.list(
                      children: [
                        _Summary(records: data.records),
                        const SizedBox(height: 14),
                        ...data.records.map(
                          (record) => Padding(
                            padding: const EdgeInsets.only(bottom: 10),
                            child: _PunchCard(record: record),
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
    child: FCard(
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

class _PunchCard extends StatelessWidget {
  const _PunchCard({required this.record});
  final BiometricRecord record;

  @override
  Widget build(BuildContext context) {
    final venue = record.venue.toUpperCase();
    final isOut = venue.contains('-OUT-');
    final label = isOut
        ? 'Exit'
        : venue.contains('-IN-')
        ? 'Entry'
        : 'Punch';
    final time = record.punchTime
        .split(':')
        .map((part) => part.padLeft(2, '0'))
        .join(':');
    return FCard(
      child: Row(
        children: [
          Icon(isOut ? FLucideIcons.logOut : FLucideIcons.logIn, size: 26),
          const SizedBox(width: 13),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                FBadge(
                  variant: isOut
                      ? FBadgeVariant.outline
                      : FBadgeVariant.secondary,
                  child: Text(label),
                ),
                const SizedBox(height: 3),
                Text(
                  record.venue,
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
              ],
            ),
          ),
          Text(
            time,
            style: const TextStyle(
              fontSize: 18,
              fontWeight: FontWeight.w800,
              fontFeatures: [FontFeature.tabularFigures()],
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
