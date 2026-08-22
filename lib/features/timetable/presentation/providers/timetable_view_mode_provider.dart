import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';

enum TimetableViewMode { daily, agenda, weekly }

class TimetableViewModeController extends Notifier<TimetableViewMode> {
  @override
  TimetableViewMode build() {
    final savedMode = ref
        .watch(settingsProvider)
        .value
        ?.getString(timetableViewModeSettingKey);
    return TimetableViewMode.values.firstWhere(
      (mode) => mode.name == savedMode,
      orElse: () => TimetableViewMode.daily,
    );
  }

  Future<void> showNext() async {
    final nextMode = switch (state) {
      TimetableViewMode.daily => TimetableViewMode.agenda,
      TimetableViewMode.agenda => TimetableViewMode.weekly,
      TimetableViewMode.weekly => TimetableViewMode.daily,
    };
    state = nextMode;
    final prefs = await ref.read(settingsProvider.future);
    await prefs.setString(timetableViewModeSettingKey, nextMode.name);
  }
}

final timetableViewModeProvider =
    NotifierProvider<TimetableViewModeController, TimetableViewMode>(
      TimetableViewModeController.new,
    );
