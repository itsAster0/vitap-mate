import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/services/class_reminder_notification_service.dart';
import 'package:vitapmate/services/exam_reminder_notification_service.dart';
part 'settings.g.dart';

@riverpod
Future<SharedPreferencesWithCache> settings(Ref ref) async {
  return SharedPreferencesWithCache.create(
    cacheOptions: SharedPreferencesWithCacheOptions(
      allowList: {
        "settings_merge_tt",
        "settings_btw_atten",
        "settings_class_notifications_enabled",
        "settings_class_notify_before_minutes",
        "settings_class_pause_until_millis",
        "settings_exam_notifications_enabled",
        "settings_exam_notify_before_minutes",
        "settings_auto_refresh_on_open",
      },
    ),
  );
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

@riverpod
bool autoRefreshOnOpen(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool("settings_auto_refresh_on_open") ?? false;
}

Future<void> setAutoRefreshOnOpen(WidgetRef ref, bool value) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool("settings_auto_refresh_on_open", value);

  ref.invalidate(autoRefreshOnOpenProvider);
}

class ClassReminderSettings {
  final bool enabled;
  final int notifyBeforeMinutes;
  final int? pauseUntilMillis;

  const ClassReminderSettings({
    required this.enabled,
    required this.notifyBeforeMinutes,
    required this.pauseUntilMillis,
  });
}

@Riverpod(keepAlive: true)
ClassReminderSettings classReminderSettings(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return ClassReminderSettings(
    enabled: prefs?.getBool("settings_class_notifications_enabled") ?? false,
    notifyBeforeMinutes:
        prefs?.getInt("settings_class_notify_before_minutes") ?? 10,
    pauseUntilMillis: prefs?.getInt("settings_class_pause_until_millis"),
  );
}

@Riverpod(keepAlive: true)
ClassReminderSettingsController classReminderSettingsController(Ref ref) {
  return ClassReminderSettingsController(ref);
}

class ClassReminderSettingsController {
  final Ref ref;
  ClassReminderSettingsController(this.ref);

  Future<void> setEnabled(bool value) async {
    final prefs = await ref.read(settingsProvider.future);
    final legacyPrefs = await SharedPreferences.getInstance();
    await prefs.setBool("settings_class_notifications_enabled", value);
    await legacyPrefs.setBool("settings_class_notifications_enabled", value);
    if (!value) {
      await ClassReminderNotificationService.cancelAll();
    }
    ref.invalidate(classReminderSettingsProvider);
  }

  Future<void> setNotifyBeforeMinutes(int value) async {
    final prefs = await ref.read(settingsProvider.future);
    final legacyPrefs = await SharedPreferences.getInstance();
    await prefs.setInt("settings_class_notify_before_minutes", value);
    await legacyPrefs.setInt("settings_class_notify_before_minutes", value);
    ref.invalidate(classReminderSettingsProvider);
  }

  Future<void> pauseForDays(int days) async {
    final prefs = await ref.read(settingsProvider.future);
    final legacyPrefs = await SharedPreferences.getInstance();
    final now = DateTime.now();
    final until = DateTime(now.year, now.month, now.day + days, 23, 59, 59);
    await prefs.setInt(
      "settings_class_pause_until_millis",
      until.millisecondsSinceEpoch,
    );
    await legacyPrefs.setInt(
      "settings_class_pause_until_millis",
      until.millisecondsSinceEpoch,
    );
    await ClassReminderNotificationService.cancelAll();
    ref.invalidate(classReminderSettingsProvider);
  }

  Future<void> clearPause() async {
    final prefs = await ref.read(settingsProvider.future);
    final legacyPrefs = await SharedPreferences.getInstance();
    await prefs.remove("settings_class_pause_until_millis");
    await legacyPrefs.remove("settings_class_pause_until_millis");
    ref.invalidate(classReminderSettingsProvider);
  }
}

class ExamReminderSettings {
  final bool enabled;
  final int notifyBeforeMinutes;

  const ExamReminderSettings({
    required this.enabled,
    required this.notifyBeforeMinutes,
  });
}

@Riverpod(keepAlive: true)
ExamReminderSettings examReminderSettings(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return ExamReminderSettings(
    enabled: prefs?.getBool("settings_exam_notifications_enabled") ?? false,
    notifyBeforeMinutes:
        prefs?.getInt("settings_exam_notify_before_minutes") ?? 10,
  );
}

@Riverpod(keepAlive: true)
ExamReminderSettingsController examReminderSettingsController(Ref ref) {
  return ExamReminderSettingsController(ref);
}

class ExamReminderSettingsController {
  final Ref ref;
  ExamReminderSettingsController(this.ref);

  Future<void> setEnabled(bool value) async {
    final prefs = await ref.read(settingsProvider.future);
    final legacyPrefs = await SharedPreferences.getInstance();
    await prefs.setBool("settings_exam_notifications_enabled", value);
    await legacyPrefs.setBool("settings_exam_notifications_enabled", value);
    if (!value) {
      await ExamReminderNotificationService.cancelAll();
    }
    ref.invalidate(examReminderSettingsProvider);
  }

  Future<void> setNotifyBeforeMinutes(int value) async {
    final prefs = await ref.read(settingsProvider.future);
    final legacyPrefs = await SharedPreferences.getInstance();
    await prefs.setInt("settings_exam_notify_before_minutes", value);
    await legacyPrefs.setInt("settings_exam_notify_before_minutes", value);
    ref.invalidate(examReminderSettingsProvider);
  }
}
