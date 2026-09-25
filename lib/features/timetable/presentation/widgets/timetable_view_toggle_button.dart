import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_view_mode_provider.dart';

/// Header button that flips between the agenda and the weekly grid. The icon
/// shows the view you'll switch to.
class TimetableViewToggleButton extends ConsumerWidget {
  const TimetableViewToggleButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final viewMode = ref.watch(timetableViewModeProvider);
    final (label, icon) = switch (viewMode) {
      TimetableViewMode.agenda => (
        'Show weekly timetable',
        FLucideIcons.columns3,
      ),
      TimetableViewMode.weekly => ('Show agenda', FLucideIcons.listTodo),
    };

    return FButton.icon(
      semanticsLabel: label,
      onPress: () => ref.read(timetableViewModeProvider.notifier).showNext(),
      child: AnimatedSwitcher(
        duration: const Duration(milliseconds: 200),
        transitionBuilder: (child, animation) => RotationTransition(
          turns: Tween(begin: 0.75, end: 1.0).animate(animation),
          child: FadeTransition(opacity: animation, child: child),
        ),
        child: Icon(icon, key: ValueKey(viewMode)),
      ),
    );
  }
}
