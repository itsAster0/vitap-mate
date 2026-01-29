import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
part 'settings.g.dart';

@riverpod
Future<SharedPreferencesWithCache> settings(Ref ref) async {
  return SharedPreferencesWithCache.create(
    cacheOptions: SharedPreferencesWithCacheOptions(
      allowList: {
        "settings_merge_tt",
        "settings_btw_atten",
        "settings_wifi_card",
      },
    ),
  );
}

@riverpod
bool wificardSetting(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool("settings_wifi_card") ?? false;
}

@riverpod
Future<void> toggleWificard(Ref ref) async {
  final prefs = await ref.read(settingsProvider.future);
  final current = prefs.getBool("settings_wifi_card") ?? true;
  await prefs.setBool("settings_wifi_card", !current);

  ref.invalidate(wificardSettingProvider);
}

@riverpod
bool mergeTT(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool("settings_merge_tt") ?? true;
}

@riverpod
Future<void> toggleMergeTT(Ref ref) async {
  final prefs = await ref.read(settingsProvider.future);

  final current = prefs.getBool("settings_merge_tt") ?? true;
  await prefs.setBool("settings_merge_tt", !current);

  ref.invalidate(mergeTTProvider);
}

@riverpod
bool btwExams(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool("settings_btw_atten") ?? false;
}

@riverpod
Future<void> toggleBTWExams(Ref ref) async {
  final prefs = await ref.read(settingsProvider.future);

  final current = prefs.getBool("settings_btw_atten") ?? false;
  await prefs.setBool("settings_btw_atten", !current);

  ref.invalidate(btwExamsProvider);
}
