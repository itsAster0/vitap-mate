import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_view_mode_provider.dart';

class TimetableViewToggleButton extends ConsumerWidget {
  const TimetableViewToggleButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final viewMode = ref.watch(timetableViewModeProvider);
    final isWeekly = viewMode == TimetableViewMode.weekly;

    return FButton.icon(
      semanticsLabel: isWeekly
          ? 'Show daily timetable'
          : 'Show weekly timetable',
      selected: isWeekly,
      onPress: () => ref.read(timetableViewModeProvider.notifier).toggle(),
      child: Icon(
        isWeekly ? Icons.view_day_outlined : Icons.view_week_outlined,
      ),
    );
  }
}
