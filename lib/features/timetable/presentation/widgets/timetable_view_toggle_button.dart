import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_view_mode_provider.dart';

class TimetableViewToggleButton extends ConsumerWidget {
  const TimetableViewToggleButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final viewMode = ref.watch(timetableViewModeProvider);
    final (label, icon) = switch (viewMode) {
      TimetableViewMode.daily => (
        'Show agenda timetable',
        Icons.view_agenda_outlined,
      ),
      TimetableViewMode.agenda => (
        'Show weekly timetable',
        Icons.view_week_outlined,
      ),
      TimetableViewMode.weekly => (
        'Show classic daily timetable',
        Icons.view_day_outlined,
      ),
    };

    return FButton.icon(
      semanticsLabel: label,
      selected: viewMode == TimetableViewMode.agenda,
      onPress: () => ref.read(timetableViewModeProvider.notifier).showNext(),
      child: Icon(icon),
    );
  }
}
