import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/docs/domain/mess_menu.dart';
import 'package:vitapmate/features/docs/presentation/providers/mess_timings_provider.dart';

/// Start of each meal, breakfast to dinner: "7:15 · 12:30 · 4:30 · 7:15".
String messTimingsSummary(MealWindows windows) {
  String start(Meal meal) =>
      mealWindowLabel(windows[meal]!).split('–').first.split(' ').first;
  return [for (final meal in Meal.values) start(meal)].join(' · ');
}

void showMessTimingsSheet(BuildContext context) {
  showFSheet(
    context: context,
    side: FLayout.btt,
    builder: (context) => const _MessTimingsSheet(),
  );
}

class _MessTimingsSheet extends ConsumerWidget {
  const _MessTimingsSheet();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final windows = ref.watch(messTimingsProvider);
    final colors = context.theme.colors;
    final typography = context.theme.typography;

    void update(Meal meal, {FTime? start, FTime? end}) {
      final current = windows[meal]!;
      final next = (
        start: start == null ? current.start : start.hour * 60 + start.minute,
        end: end == null ? current.end : end.hour * 60 + end.minute,
      );
      if (next.end <= next.start) {
        dispToast(
          context,
          'Check the times',
          '${meal.label} has to end after it starts.',
        );
        return;
      }
      setMessTimings(ref, {...windows, meal: next});
    }

    return Container(
      decoration: BoxDecoration(
        color: colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      ),
      child: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(Space.lg, Space.md, Space.lg, 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const SheetHandle(),
              const SizedBox(height: Space.md),
              Text(
                'Mess Timings',
                style: typography.body.lg.copyWith(
                  fontWeight: FontWeight.w700,
                  color: colors.foreground,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                'Marks the meal on now in the mess menu.',
                style: typography.body.sm.copyWith(
                  color: colors.mutedForeground,
                ),
              ),
              const SizedBox(height: Space.lg),
              for (final meal in Meal.values)
                Padding(
                  padding: const EdgeInsets.only(bottom: Space.sm + 2),
                  child: Row(
                    children: [
                      Expanded(
                        child: Text(
                          meal.label,
                          style: typography.body.sm.copyWith(
                            fontWeight: FontWeight.w600,
                            color: colors.foreground,
                          ),
                        ),
                      ),
                      _TimeButton(
                        minutes: windows[meal]!.start,
                        title: '${meal.label} starts',
                        onPicked: (t) => update(meal, start: t),
                      ),
                      Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: Space.sm,
                        ),
                        child: Text(
                          '–',
                          style: typography.body.sm.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                      ),
                      _TimeButton(
                        minutes: windows[meal]!.end,
                        title: '${meal.label} ends',
                        onPicked: (t) => update(meal, end: t),
                      ),
                    ],
                  ),
                ),
              const SizedBox(height: Space.sm),
              FButton(
                variant: FButtonVariant.outline,
                onPress: () => resetMessTimings(ref),
                child: const Text('Reset to VIT-AP timings'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// A time shown as a field; tapping opens a wheel picker in a dialog.
class _TimeButton extends StatelessWidget {
  const _TimeButton({
    required this.minutes,
    required this.title,
    required this.onPicked,
  });

  final int minutes;
  final String title;
  final ValueChanged<FTime> onPicked;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final initial = FTime(minutes ~/ 60, minutes % 60);
    final label = DateFormat(
      'h:mm a',
    ).format(DateTime(2000, 1, 1, initial.hour, initial.minute));

    Future<void> pick() async {
      var picked = initial;
      await showFDialog(
        context: context,
        builder: (context, style, animation) => AppDialog(
          animation: animation,
          direction: Axis.horizontal,
          title: Text(title),
          body: SizedBox(
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
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(context).pop(),
              child: const Text('Cancel'),
            ),
            FButton(
              onPress: () {
                Navigator.of(context).pop();
                onPicked(picked);
              },
              child: const Text('Save'),
            ),
          ],
        ),
      );
    }

    return PressScale(
      onPress: pick,
      semanticsLabel: '$title, $label',
      child: Container(
        width: 104,
        height: 40,
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: colors.card,
          borderRadius: BorderRadius.circular(Radii.md),
          border: Border.all(color: colors.border),
        ),
        child: Text(
          label,
          style: context.theme.typography.body.sm.copyWith(
            fontWeight: FontWeight.w500,
            color: colors.foreground,
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
      ),
    );
  }
}
