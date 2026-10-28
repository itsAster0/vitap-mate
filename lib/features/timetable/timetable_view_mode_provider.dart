import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/app/settings.dart';

enum TimetableViewMode { agenda, weekly }

class TimetableViewModeController extends Notifier<TimetableViewMode> {
  @override
  TimetableViewMode build() {
    final savedMode = ref
        .watch(settingsProvider)
        .value
        ?.getString(timetableViewModeSettingKey);
    // Anything else, including the removed "daily" view, opens the agenda.
    return TimetableViewMode.values.asNameMap()[savedMode] ?? .agenda;
  }

  Future<void> showNext() async {
    state = switch (state) {
      .agenda => .weekly,
      .weekly => .agenda,
    };
    final prefs = await ref.read(settingsProvider.future);
    await prefs.setString(timetableViewModeSettingKey, state.name);
  }
}

final timetableViewModeProvider =
    NotifierProvider<TimetableViewModeController, TimetableViewMode>(
      TimetableViewModeController.new,
    );
