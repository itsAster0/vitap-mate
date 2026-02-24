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
        "settings_wifi_card",
        "settings_class_notifications_enabled",
        "settings_class_notify_before_minutes",
        "settings_class_pause_until_millis",
        "settings_exam_notifications_enabled",
        "settings_exam_notify_before_minutes",
        "settings_student_projects_pinned_ids",
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

final studentProjectPinnedIdsProvider = Provider<Set<int>>((ref) {
  final prefs = ref.watch(settingsProvider).value;
  final list =
      prefs?.getStringList("settings_student_projects_pinned_ids") ?? [];
  return list.map(int.tryParse).whereType<int>().toSet();
});

final studentProjectsPinnedOnlySessionProvider = StateProvider<bool>(
  (ref) => false,
);

final studentProjectsSettingsControllerProvider =
    Provider<StudentProjectsSettingsController>((ref) {
      return StudentProjectsSettingsController(ref);
    });

class StudentProjectsSettingsController {
  final Ref ref;
  StudentProjectsSettingsController(this.ref);

  Future<void> togglePinned(int id) async {
    final prefs = await ref.read(settingsProvider.future);
    final current =
        (prefs.getStringList("settings_student_projects_pinned_ids") ?? [])
            .map(int.tryParse)
            .whereType<int>()
            .toSet();
    if (current.contains(id)) {
      current.remove(id);
    } else {
      current.add(id);
    }
    final sorted = current.toList()..sort();
    await prefs.setStringList(
      "settings_student_projects_pinned_ids",
      sorted.map((e) => "$e").toList(),
    );
    ref.invalidate(studentProjectPinnedIdsProvider);
  }
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

final classReminderSettingsProvider = Provider<ClassReminderSettings>((ref) {
  final prefs = ref.watch(settingsProvider).value;
  return ClassReminderSettings(
    enabled: prefs?.getBool("settings_class_notifications_enabled") ?? false,
    notifyBeforeMinutes:
        prefs?.getInt("settings_class_notify_before_minutes") ?? 10,
    pauseUntilMillis: prefs?.getInt("settings_class_pause_until_millis"),
  );
});

final classReminderSettingsControllerProvider =
    Provider<ClassReminderSettingsController>((ref) {
      return ClassReminderSettingsController(ref);
    });

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

final examReminderSettingsProvider = Provider<ExamReminderSettings>((ref) {
  final prefs = ref.watch(settingsProvider).value;
  return ExamReminderSettings(
    enabled: prefs?.getBool("settings_exam_notifications_enabled") ?? false,
    notifyBeforeMinutes:
        prefs?.getInt("settings_exam_notify_before_minutes") ?? 10,
  );
});

final examReminderSettingsControllerProvider =
    Provider<ExamReminderSettingsController>((ref) {
      return ExamReminderSettingsController(ref);
    });

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
