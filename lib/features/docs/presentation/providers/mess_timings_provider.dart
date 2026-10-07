import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/features/docs/domain/mess_menu.dart';

/// Mess serving hours, set in Settings → Mess Timings.
final messTimingsProvider = Provider<MealWindows>((ref) {
  final prefs = ref.watch(settingsProvider).value;
  return decodeMealWindows(prefs?.getString(messTimingsSettingKey));
});

Future<void> setMessTimings(WidgetRef ref, MealWindows windows) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setString(messTimingsSettingKey, encodeMealWindows(windows));
  ref.invalidate(messTimingsProvider);
}

Future<void> resetMessTimings(WidgetRef ref) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.remove(messTimingsSettingKey);
  ref.invalidate(messTimingsProvider);
}
