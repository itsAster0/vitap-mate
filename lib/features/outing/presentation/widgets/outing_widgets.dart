import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/widgets/fit_calendar.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/outing/domain/outing_rules.dart';

Tone statusTone(BuildContext context, OutingStatus status) {
  final colors = context.theme.colors;
  return switch (status) {
    OutingStatus.approved => colors.app.success,
    OutingStatus.pending => colors.app.warning,
    OutingStatus.rejected => colors.app.danger,
    OutingStatus.other => Tone(
      base: colors.mutedForeground,
      subtle: colors.secondary,
      onSubtle: colors.mutedForeground,
    ),
  };
}

String statusLabel(OutingStatus status, String raw) => switch (status) {
  OutingStatus.approved => 'APPROVED',
  OutingStatus.pending => 'PENDING',
  OutingStatus.rejected => 'REJECTED',
  OutingStatus.other => raw.toUpperCase(),
};

/// Who still has to approve a pending request: "Waiting for Warden's
/// Approval" → "Warden".
String? pendingWith(String status) => RegExp(
  r"waiting for (\w+)",
  caseSensitive: false,
).firstMatch(status)?.group(1);

/// Small caption above a group of fields.
class FieldLabel extends StatelessWidget {
  const FieldLabel(this.text, {super.key, this.trailing});

  final String text;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.sm - 2, left: 2),
      child: Row(
        children: [
          Expanded(
            child: Text(
              text,
              style: context.theme.typography.body.sm.copyWith(
                fontWeight: FontWeight.w600,
                color: colors.foreground,
              ),
            ),
          ),
          if (trailing != null)
            DefaultTextStyle.merge(
              style: context.theme.typography.body.xs.copyWith(
                color: colors.mutedForeground,
              ),
              child: trailing!,
            ),
        ],
      ),
    );
  }
}

/// A field's problem, in the danger tone, or nothing.
class FieldError extends StatelessWidget {
  const FieldError(this.message, {super.key});

  final String? message;

  @override
  Widget build(BuildContext context) {
    return AnimatedSize(
      duration: Motion.fast,
      alignment: Alignment.topLeft,
      child: message == null
          ? const SizedBox(width: double.infinity)
          : Padding(
              padding: const EdgeInsets.only(top: Space.xs + 2, left: 2),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Padding(
                    padding: const EdgeInsets.only(top: 2),
                    child: Icon(
                      FLucideIcons.circleAlert,
                      size: 12,
                      color: context.theme.colors.app.danger.base,
                    ),
                  ),
                  const SizedBox(width: Space.xs + 2),
                  Expanded(
                    child: Text(
                      message!,
                      style: context.theme.typography.body.xs.copyWith(
                        color: context.theme.colors.app.danger.base,
                      ),
                    ),
                  ),
                ],
              ),
            ),
    );
  }
}

/// A selectable pill, like the biometric date chips.
class ChoicePill extends StatelessWidget {
  const ChoicePill({
    super.key,
    required this.label,
    required this.selected,
    required this.onPress,
    this.caption,
  });

  final String label;
  final String? caption;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final fg = selected ? colors.primaryForeground : colors.foreground;
    return PressScale(
      scale: 0.95,
      semanticsLabel: caption == null ? label : '$caption $label',
      onPress: onPress,
      child: AnimatedContainer(
        duration: Motion.medium,
        padding: const EdgeInsets.symmetric(
          horizontal: Space.md,
          vertical: Space.sm,
        ),
        decoration: BoxDecoration(
          color: selected ? colors.primary : colors.card,
          borderRadius: BorderRadius.circular(Radii.md),
          border: Border.all(color: selected ? colors.primary : colors.border),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (caption != null)
              Text(
                caption!,
                style: context.theme.typography.body.xs.copyWith(
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.3,
                  color: selected
                      ? colors.primaryForeground.withValues(alpha: 0.75)
                      : colors.mutedForeground,
                ),
              ),
            FittedBox(
              fit: BoxFit.scaleDown,
              child: Text(
                label,
                maxLines: 1,
                style: context.theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w600,
                  color: fg,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Looks like a field; tapping it opens a picker.
class PickerField extends StatelessWidget {
  const PickerField({
    super.key,
    required this.icon,
    required this.text,
    required this.placeholder,
    required this.onPress,
    this.error = false,
  });

  final IconData icon;
  final String? text;
  final String placeholder;
  final VoidCallback onPress;
  final bool error;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return PressScale(
      scale: 0.97,
      onPress: onPress,
      semanticsLabel: text ?? placeholder,
      child: AnimatedContainer(
        duration: Motion.fast,
        height: 42,
        padding: const EdgeInsets.symmetric(horizontal: Space.md),
        decoration: BoxDecoration(
          color: colors.card,
          borderRadius: BorderRadius.circular(Radii.md),
          border: Border.all(
            color: error ? colors.app.danger.base : colors.border,
          ),
        ),
        child: Row(
          children: [
            Icon(icon, size: 15, color: colors.mutedForeground),
            const SizedBox(width: Space.sm),
            Expanded(
              child: Text(
                text ?? placeholder,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: context.theme.typography.body.sm.copyWith(
                  fontWeight: text == null ? FontWeight.w400 : FontWeight.w500,
                  color: text == null
                      ? colors.mutedForeground
                      : colors.foreground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Picks a day between [first] and [last] in a calendar dialog.
Future<DateTime?> pickOutingDate(
  BuildContext context, {
  required String title,
  required DateTime first,
  required DateTime last,
  DateTime? initial,
}) {
  // Otherwise the last text field takes focus back and the keyboard opens.
  FocusManager.instance.primaryFocus?.unfocus();
  final start = initial == null || initial.isBefore(first) ? first : initial;
  final today = dateOnly(DateTime.now());
  return showFDialog<DateTime>(
    context: context,
    // Above the tab shell, so the keyboard inset is counted once.
    useRootNavigator: true,
    useSafeArea: true,
    builder: (dialogContext, _, animation) => AppDialog(
      animation: animation,
      title: Text(title),
      // The dialog already frames it, so no second border around the grid.
      body: FitCalendar(
        control: FGridCalendarControl(
          start: first,
          // Inclusive: [last] itself can be picked.
          end: last,
          initial: start,
          // forui asserts today lies in [start, end]; a return date picker
          // starts on the leaving day, which can be after today.
          today: today.isBefore(first)
              ? first
              : today.isAfter(last)
              ? last
              : today,
        ),
        selectionControl: FDateSelectionControl.liftedSingle(
          value: initial,
          onChange: (_) {},
        ),
        onDayPress: (date) => Navigator.of(dialogContext).pop(date),
      ),
      actions: [
        FButton(
          variant: FButtonVariant.outline,
          onPress: () => Navigator.of(dialogContext).pop(),
          child: const Text('Cancel'),
        ),
      ],
    ),
  );
}

/// Picks a time of day on a wheel, starting at [initial].
Future<FTime?> pickOutingTime(
  BuildContext context, {
  required String title,
  required FTime initial,
  String? hint,
}) {
  FocusManager.instance.primaryFocus?.unfocus();
  var picked = initial;
  return showFDialog<FTime>(
    context: context,
    // Above the tab shell, so the keyboard inset is counted once.
    useRootNavigator: true,
    builder: (dialogContext, _, animation) => AppDialog(
      animation: animation,
      direction: Axis.horizontal,
      title: Text(title),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (hint != null)
            Padding(
              padding: const EdgeInsets.only(bottom: Space.sm),
              child: Text(
                hint,
                style: dialogContext.theme.typography.body.xs.copyWith(
                  color: dialogContext.theme.colors.mutedForeground,
                ),
              ),
            ),
          SizedBox(
            height: 180,
            child: FTimePicker(
              hour24: false,
              minuteInterval: 5,
              control: FTimePickerControl.managed(
                initial: initial,
                onChange: (value) => picked = value,
              ),
            ),
          ),
        ],
      ),
      actions: [
        FButton(
          variant: FButtonVariant.outline,
          onPress: () => Navigator.of(dialogContext).pop(),
          child: const Text('Cancel'),
        ),
        FButton(
          onPress: () => Navigator.of(dialogContext).pop(picked),
          child: const Text('Set'),
        ),
      ],
    ),
  );
}

/// The rules VTOP applies, one fact per cell (a small label over a short
/// value), and the button to apply. VTOP's own notice shows only while it
/// is not taking applications.
class OutingRulesPanel extends StatelessWidget {
  const OutingRulesPanel({
    super.key,
    required this.open,
    required this.facts,
    required this.onApply,
    this.notice,
  });

  final bool open;
  final String? notice;

  /// (label, value) pairs, laid out two per row.
  final List<(String, String)> facts;
  final VoidCallback onApply;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final rows = [
      for (var i = 0; i < facts.length; i += 2)
        facts.sublist(i, i + 2 > facts.length ? facts.length : i + 2),
    ];
    return Surface(
      padding: const EdgeInsets.all(Space.md + 2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (!open) ...[
            Text(
              'Applications are closed',
              style: typography.body.sm.copyWith(
                fontWeight: FontWeight.w700,
                color: colors.foreground,
              ),
            ),
            if (notice != null && notice!.isNotEmpty) ...[
              const SizedBox(height: 2),
              Text(
                notice!,
                style: typography.body.xs.copyWith(
                  color: colors.mutedForeground,
                ),
              ),
            ],
            const SizedBox(height: Space.md),
          ],
          for (final (i, row) in rows.indexed) ...[
            if (i > 0) const SizedBox(height: Space.md),
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final (j, (label, value)) in row.indexed)
                  Expanded(
                    child: Padding(
                      padding: EdgeInsets.only(left: j == 0 ? 0 : Space.md),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            label,
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                            ),
                          ),
                          const SizedBox(height: 1),
                          Text(
                            value,
                            style: typography.body.sm.copyWith(
                              fontWeight: FontWeight.w600,
                              color: colors.foreground,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                if (row.length == 1) const Spacer(),
              ],
            ),
          ],
          const SizedBox(height: Space.md + 2),
          FButton(
            onPress: open ? onApply : null,
            prefix: const Icon(FLucideIcons.plus),
            child: Text(open ? 'New request' : 'Applications closed'),
          ),
        ],
      ),
    );
  }
}

/// One outing request: where, why, when, its status and the pass.
class OutingRequestCard extends StatelessWidget {
  const OutingRequestCard({
    super.key,
    required this.place,
    required this.purpose,
    required this.when,
    required this.status,
    required this.highlight,
    this.onPass,
    this.passLoading = false,
    this.onCancel,
    this.cancelLoading = false,
  });

  final String place;
  final String purpose;

  /// When, as lines with an optional label: `[(null, 'Sun 11 Oct · …')]` or
  /// `[('Out', …), ('Back', …)]`.
  final List<(String?, String)> when;
  final String status;

  /// "OUT NOW", "TOMORROW", "IN 3 DAYS" for requests still ahead.
  final String? highlight;
  final VoidCallback? onPass;
  final bool passLoading;

  /// Set while VTOP still lets the request be cancelled.
  final VoidCallback? onCancel;
  final bool cancelLoading;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final kind = outingStatus(status);
    final tone = statusTone(context, kind);
    final waitingOn = kind == OutingStatus.pending ? pendingWith(status) : null;
    return Surface(
      padding: const EdgeInsets.fromLTRB(
        Space.md + 2,
        Space.md,
        Space.md,
        Space.md,
      ),
      borderColor: highlight != null ? colors.app.accentTone.base : null,
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Flexible(
                      child: Text(
                        place,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: context.theme.typography.body.md.copyWith(
                          fontWeight: FontWeight.w700,
                          color: colors.foreground,
                        ),
                      ),
                    ),
                    const SizedBox(width: Space.sm),
                    ToneBadge(label: statusLabel(kind, status), tone: tone),
                    if (highlight != null) ...[
                      const SizedBox(width: Space.xs),
                      ToneBadge(label: highlight!, tone: colors.app.accentTone),
                    ],
                  ],
                ),
                if (purpose.isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text(
                    purpose,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: context.theme.typography.body.sm.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                ],
                const SizedBox(height: Space.xs + 2),
                for (final (label, text) in when)
                  Row(
                    children: [
                      SizedBox(
                        width: label == null ? 17 : 38,
                        child: label == null
                            ? Icon(
                                FLucideIcons.clock,
                                size: 12,
                                color: colors.mutedForeground,
                              )
                            : Text(
                                label,
                                style: context.theme.typography.body.xs
                                    .copyWith(color: colors.mutedForeground),
                              ),
                      ),
                      Expanded(
                        child: Text(
                          text,
                          style: context.theme.typography.body.xs.copyWith(
                            fontWeight: FontWeight.w500,
                            color: colors.secondaryForeground,
                            fontFeatures: const [FontFeature.tabularFigures()],
                          ),
                        ),
                      ),
                    ],
                  ),
                if (waitingOn != null) ...[
                  const SizedBox(height: 2),
                  Text(
                    'Waiting on your $waitingOn',
                    style: context.theme.typography.body.xs.copyWith(
                      color: tone.onSubtle,
                    ),
                  ),
                ],
              ],
            ),
          ),
          if (onPass != null) ...[
            const SizedBox(width: Space.sm),
            SizedBox(
              width: 64,
              child: FButton(
                variant: FButtonVariant.outline,
                size: FButtonSizeVariant.sm,
                onPress: passLoading ? null : onPass,
                child: passLoading
                    ? const SizedBox.square(
                        dimension: 14,
                        child: FCircularProgress(),
                      )
                    : const Text('Pass'),
              ),
            ),
          ],
          if (onPass == null && onCancel != null) ...[
            const SizedBox(width: Space.sm),
            SizedBox(
              width: 84,
              child: FButton(
                variant: FButtonVariant.destructive,
                size: FButtonSizeVariant.sm,
                onPress: cancelLoading ? null : onCancel,
                child: cancelLoading
                    ? const SizedBox.square(
                        dimension: 14,
                        child: FCircularProgress(),
                      )
                    : const Text('Cancel'),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// "OUT NOW", "TODAY", "TOMORROW" or "IN 3 DAYS" for a trip from [start] to
/// [end] that has not finished; nothing for one in the past.
String? upcomingLabel(DateTime start, DateTime end, DateTime now) {
  if (end.isBefore(now)) return null;
  if (!start.isAfter(now)) return 'OUT NOW';
  final days = dateOnly(start).difference(dateOnly(now)).inDays;
  return switch (days) {
    0 => 'TODAY',
    1 => 'TOMORROW',
    _ => 'IN $days DAYS',
  };
}

/// `Sat 10 Oct` (with the year when it is not this year).
String shortDate(DateTime date, DateTime now) => DateFormat(
  date.year == now.year ? 'EEE d MMM' : 'EEE d MMM yyyy',
  'en_US',
).format(date);
