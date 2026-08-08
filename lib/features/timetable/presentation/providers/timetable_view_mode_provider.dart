import 'package:flutter_riverpod/flutter_riverpod.dart';

enum TimetableViewMode { daily, weekly }

class TimetableViewModeController extends Notifier<TimetableViewMode> {
  @override
  TimetableViewMode build() => TimetableViewMode.daily;

  void toggle() {
    state = state == TimetableViewMode.daily
        ? TimetableViewMode.weekly
        : TimetableViewMode.daily;
  }
}

final timetableViewModeProvider =
    NotifierProvider.autoDispose<
      TimetableViewModeController,
      TimetableViewMode
    >(TimetableViewModeController.new);
