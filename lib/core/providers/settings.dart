import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
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
        "settings_auto_refresh",
        "settings_class_notifications_enabled",
        "settings_class_notify_before_minutes",
        "settings_class_pause_until_millis",
        "settings_exam_notifications_enabled",
        "settings_exam_notify_before_minutes",
        "settings_student_projects_pinned_ids",
        "settings_student_projects_json",
        "settings_student_projects_rotation_seed",
      },
    ),
  );
}

@riverpod
bool mergeTT(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool("settings_merge_tt") ?? true;
}

Future<void> setMergeTT(WidgetRef ref, bool value) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool("settings_merge_tt", value);
  ref.invalidate(mergeTTProvider);
}

@riverpod
bool btwExams(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool("settings_btw_atten") ?? false;
}

Future<void> setbtwExam(WidgetRef ref, bool value) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool("settings_btw_atten", value);
  ref.invalidate(btwExamsProvider);
}

@riverpod
bool autoRefresh(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return prefs?.getBool("settings_auto_refresh") ?? false;
}

Future<void> setautoRefresh(WidgetRef ref, bool value) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool("settings_auto_refresh", value);
  ref.invalidate(autoRefreshProvider);
}

@riverpod
Set<int> studentProjectPinnedIds(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  final list =
      prefs?.getStringList("settings_student_projects_pinned_ids") ?? [];
  return list.map(int.tryParse).whereType<int>().toSet();
}

@riverpod
class StudentProjectsPinnedOnlySession
    extends _$StudentProjectsPinnedOnlySession {
  @override
  bool build() => false;

  void setValue(bool value) {
    state = value;
  }
}

@riverpod
StudentProjectsSettingsController studentProjectsSettingsController(Ref ref) {
  return StudentProjectsSettingsController(ref);
}

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

@riverpod
ClassReminderSettings classReminderSettings(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return ClassReminderSettings(
    enabled: prefs?.getBool("settings_class_notifications_enabled") ?? false,
    notifyBeforeMinutes:
        prefs?.getInt("settings_class_notify_before_minutes") ?? 10,
    pauseUntilMillis: prefs?.getInt("settings_class_pause_until_millis"),
  );
}

@riverpod
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

@riverpod
ExamReminderSettings examReminderSettings(Ref ref) {
  final prefs = ref.watch(settingsProvider).value;
  return ExamReminderSettings(
    enabled: prefs?.getBool("settings_exam_notifications_enabled") ?? false,
    notifyBeforeMinutes:
        prefs?.getInt("settings_exam_notify_before_minutes") ?? 10,
  );
}

@riverpod
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
