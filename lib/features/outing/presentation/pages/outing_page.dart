import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/utils/vtop_client_call.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/outing/data/outing_service.dart';
import 'package:vitapmate/features/outing/domain/outing_rules.dart';
import 'package:vitapmate/features/outing/presentation/widgets/general_apply_sheet.dart';
import 'package:vitapmate/features/outing/presentation/widgets/outing_widgets.dart';
import 'package:vitapmate/features/outing/presentation/widgets/weekend_apply_sheet.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum OutingTab { general, weekend }

class OutingPage extends HookConsumerWidget {
  const OutingPage({super.key, this.initialTab = OutingTab.general});

  final OutingTab initialTab;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tab = useState(initialTab);
    final colors = context.theme.colors;

    Future<void> refresh() async {
      final provider = tab.value == OutingTab.general
          ? generalOutingProvider
          : weekendOutingProvider;
      ref.invalidate(provider);
      try {
        await ref.read(provider.future);
      } catch (e) {
        log('outing refresh failed: $e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    return ScreenRefresh(
      onRefresh: refresh,
      child: RefreshIndicator(
        onRefresh: refresh,
        backgroundColor: colors.background,
        color: colors.foreground,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(
            Space.sm,
            Space.sm,
            Space.sm,
            Space.xl,
          ),
          children: [
            Segmented<OutingTab>(
              segments: const [
                (OutingTab.general, 'General'),
                (OutingTab.weekend, 'Weekend'),
              ],
              value: tab.value,
              onChanged: (value) => tab.value = value,
            ),
            const SizedBox(height: Space.md),
            AnimatedSwitcher(
              duration: Motion.medium,
              child: tab.value == OutingTab.general
                  ? const _GeneralTab(key: ValueKey('general'))
                  : const _WeekendTab(key: ValueKey('weekend')),
            ),
          ],
        ),
      ),
    );
  }
}

class _GeneralTab extends ConsumerWidget {
  const _GeneralTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data = ref.watch(generalOutingProvider);
    return data.when(
      skipLoadingOnRefresh: true,
      loading: () => const _Loading(),
      error: (error, _) =>
          _LoadError(onRetry: () => ref.invalidate(generalOutingProvider)),
      data: (form) {
        final now = DateTime.now();
        final outWindow = offeredWindow(form.outHours);
        final inWindow = offeredWindow(form.inHours);
        final records = [...form.records]
          ..sort(
            (a, b) => (parseOutingMoment(b.fromDate) ?? DateTime(0)).compareTo(
              parseOutingMoment(a.fromDate) ?? DateTime(0),
            ),
          );
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            OutingRulesPanel(
              open: form.student != null,
              notice: form.notice,
              facts: [
                ('Apply', '24 h – ${form.maxDaysAhead} days ahead'),
                if (outWindow != null)
                  (
                    'Leave',
                    '${shortClock(outWindow.earliest)} – '
                        '${shortClock(outWindow.latest)}',
                  ),
                if (inWindow != null) ('Back by', shortClock(inWindow.latest)),
                ('Longest trip', '${form.maxDaysAway} days'),
              ],
              onApply: () async {
                final result = await showGeneralApplySheet(context, form);
                if (result != null && context.mounted) {
                  dispToast(context, 'Request sent', result.message);
                  ref.invalidate(generalOutingProvider);
                }
              },
            ),
            _RequestList(
              count: records.length,
              children: [
                for (final record in records)
                  _GeneralRequest(record: record, now: now),
              ],
            ),
          ],
        );
      },
    );
  }
}

class _GeneralRequest extends StatelessWidget {
  const _GeneralRequest({required this.record, required this.now});

  final GeneralOutingRecord record;
  final DateTime now;

  @override
  Widget build(BuildContext context) {
    final start = parseOutingMoment(record.fromDate, record.fromTime);
    final end = parseOutingMoment(record.toDate, record.toTime);
    final endOfDay = end == null
        ? null
        : record.toTime.isEmpty
        ? end.add(const Duration(hours: 23, minutes: 59))
        : end;
    final kind = outingStatus(record.status);
    return _PassCard(
      weekend: false,
      passId: record.passId,
      passName: record.passId,
      cancelId: record.cancelId,
      place: record.place,
      purpose: record.purpose,
      status: record.status,
      when: _generalWhen(start, end, record),
      highlight:
          start != null && endOfDay != null && kind != OutingStatus.rejected
          ? upcomingLabel(start, endOfDay, now)
          : null,
    );
  }

  List<(String?, String)> _generalWhen(
    DateTime? start,
    DateTime? end,
    GeneralOutingRecord record,
  ) {
    if (start == null || end == null) {
      return [
        ('Out', '${record.fromDate} ${record.fromTime}'),
        ('Back', '${record.toDate} ${record.toTime}'),
      ];
    }
    String time(DateTime moment, String raw) => raw.isEmpty
        ? ''
        : ', ${clockLabel(dayMinutes(moment.hour, moment.minute))}';
    final away = awayLabel(start, end, hasReturnTime: record.toTime.isNotEmpty);
    if (dateOnly(start) == dateOnly(end)) {
      final back = record.toTime.isEmpty
          ? ''
          : ' – ${clockLabel(dayMinutes(end.hour, end.minute))}';
      return [
        (null, '${shortDate(start, now)}${time(start, record.fromTime)}$back'),
        if (away != null) ('Away', away),
      ];
    }
    return [
      ('Out', '${shortDate(start, now)}${time(start, record.fromTime)}'),
      ('Back', '${shortDate(end, now)}${time(end, record.toTime)}'),
      if (away != null) ('Away', away),
    ];
  }
}

class _WeekendTab extends ConsumerWidget {
  const _WeekendTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data = ref.watch(weekendOutingProvider);
    return data.when(
      skipLoadingOnRefresh: true,
      loading: () => const _Loading(),
      error: (error, _) =>
          _LoadError(onRetry: () => ref.invalidate(weekendOutingProvider)),
      data: (form) {
        final now = DateTime.now();
        final dates = weekendOutingDates(form, now);
        final open = form.student != null;
        final records = [...form.records]
          ..sort(
            (a, b) => (parseOutingMoment(b.date) ?? DateTime(0)).compareTo(
              parseOutingMoment(a.date) ?? DateTime(0),
            ),
          );
        final dayFormat = DateFormat('EEE d MMM', 'en_US');
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            OutingRulesPanel(
              open: open && dates.isNotEmpty,
              notice: !open ? form.notice : 'No outing day in the coming week.',
              facts: [
                ('Outing days', weekdayShortNames(form.weekdays)),
                ('Next', dates.isEmpty ? '—' : dayFormat.format(dates.first)),
                ('Time slots', '${form.timeSlots.length}'),
                ('Places', '${form.places.length}'),
              ],
              onApply: () async {
                final result = await showWeekendApplySheet(context, form);
                if (result != null && context.mounted) {
                  dispToast(context, 'Request sent', result.message);
                  ref.invalidate(weekendOutingProvider);
                }
              },
            ),
            _RequestList(
              count: records.length,
              children: [
                for (final record in records)
                  _WeekendRequest(record: record, now: now),
              ],
            ),
          ],
        );
      },
    );
  }
}

class _WeekendRequest extends StatelessWidget {
  const _WeekendRequest({required this.record, required this.now});

  final WeekendOutingRecord record;
  final DateTime now;

  @override
  Widget build(BuildContext context) {
    final day = parseOutingMoment(record.date);
    final slot = parseTimeSlot(record.timeSlot);
    String? highlight;
    if (day != null &&
        slot != null &&
        outingStatus(record.status) != OutingStatus.rejected) {
      highlight = upcomingLabel(
        day.add(Duration(minutes: slot.start)),
        day.add(Duration(minutes: slot.end)),
        now,
      );
    }
    return _PassCard(
      weekend: true,
      passId: record.passId,
      passName: record.passId,
      cancelId: record.cancelId,
      place: record.place,
      purpose: record.purpose,
      status: record.status,
      when: [
        (
          null,
          [
            if (day != null) shortDate(day, now) else record.date,
            slotLabel(record.timeSlot),
          ].join(' · '),
        ),
      ],
      highlight: highlight,
    );
  }
}

/// A request card whose Pass button saves the pass to Docs and opens it.
class _PassCard extends HookConsumerWidget {
  const _PassCard({
    required this.weekend,
    required this.passId,
    required this.passName,
    required this.cancelId,
    required this.place,
    required this.purpose,
    required this.status,
    required this.when,
    required this.highlight,
  });

  final bool weekend;
  final String passId;
  final String passName;
  final String cancelId;
  final String place;
  final String purpose;
  final String status;
  final List<(String?, String)> when;
  final String? highlight;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final loading = useState(false);
    final cancelling = useState(false);

    Future<void> cancel() async {
      final sure = await showFDialog<bool>(
        context: context,
        useRootNavigator: true,
        builder: (dialogContext, _, animation) => AppDialog(
          animation: animation,
          title: const Text('Cancel this request?'),
          body: Text(
            '$place · $purpose\nIt is removed from VTOP. To go later, '
            'you will have to apply again.',
          ),
          actions: [
            FButton(
              variant: FButtonVariant.destructive,
              onPress: () => Navigator.of(dialogContext).pop(true),
              child: const Text('Cancel request'),
            ),
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(dialogContext).pop(false),
              child: const Text('Keep it'),
            ),
          ],
        ),
      );
      if (sure != true || !context.mounted) return;
      cancelling.value = true;
      try {
        final result = await ref
            .read(outingServiceProvider)
            .cancel(weekend: weekend, cancelId: cancelId);
        if (!context.mounted) return;
        dispToast(
          context,
          result.cancelled ? 'Request cancelled' : "Couldn't cancel",
          result.message,
        );
        ref.invalidate(weekend ? weekendOutingProvider : generalOutingProvider);
      } catch (e) {
        log('outing cancel failed: $e');
        if (context.mounted) {
          dispToast(context, "Couldn't cancel", vtopErrorMessage(e));
        }
      } finally {
        if (context.mounted) cancelling.value = false;
      }
    }

    Future<void> openPass() async {
      loading.value = true;
      try {
        final doc = await ref
            .read(outingServiceProvider)
            .pass(weekend: weekend, passId: passId, name: passName);
        if (context.mounted) {
          GoRouter.of(context).pushNamed(Paths.outingPass, extra: doc);
        }
      } catch (e) {
        log('outing pass failed: $e');
        if (context.mounted) {
          dispToast(context, "Couldn't get the pass", vtopErrorMessage(e));
        }
      } finally {
        if (context.mounted) loading.value = false;
      }
    }

    return OutingRequestCard(
      place: place,
      purpose: purpose,
      when: when,
      status: status,
      highlight: highlight,
      onPass: passId.isEmpty ? null : openPass,
      passLoading: loading.value,
      onCancel: cancelId.isEmpty ? null : cancel,
      cancelLoading: cancelling.value,
    );
  }
}

class _RequestList extends StatelessWidget {
  const _RequestList({required this.count, required this.children});

  final int count;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SectionHeader(title: 'Your requests', trailing: Text('$count')),
        if (children.isEmpty)
          const EmptyState(
            icon: FLucideIcons.doorOpen,
            title: 'No requests yet',
            message: 'Requests you send show here with their status.',
          )
        else
          for (final (i, child) in children.indexed)
            Padding(
              padding: const EdgeInsets.only(bottom: Space.sm),
              child: EnterFade(index: i, child: child),
            ),
      ],
    );
  }
}

class _Loading extends StatelessWidget {
  const _Loading();

  @override
  Widget build(BuildContext context) => const Column(
    children: [
      Skeleton(height: 150, radius: Radii.lg),
      SizedBox(height: Space.lg),
      SkeletonList(count: 4, height: 88),
    ],
  );
}

class _LoadError extends StatelessWidget {
  const _LoadError({required this.onRetry});

  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) => EmptyState(
    icon: FLucideIcons.cloudOff,
    title: "Couldn't load outings",
    message: 'VTOP did not answer. Check your connection and retry.',
    action: FButton(
      variant: FButtonVariant.outline,
      prefix: const Icon(FLucideIcons.rotateCw),
      onPress: onRetry,
      mainAxisSize: MainAxisSize.min,
      child: const Text('Retry'),
    ),
  );
}
