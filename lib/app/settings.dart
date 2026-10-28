import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:hooks_riverpod/misc.dart' show ProviderOrFamily;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/platform/notifications/change_alert_notification_service.dart';
import 'package:vitapmate/platform/notifications/class_reminder_notification_service.dart';
import 'package:vitapmate/platform/notifications/exam_reminder_notification_service.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';

const emailOtpDeleteAfterReadingSettingKey =
    'settings_email_otp_delete_after_reading';
const timetableViewModeSettingKey = 'settings_timetable_view_mode';
const emailOtpBannerDismissedSettingKey = 'settings_email_otp_banner_dismissed';
const refreshButtonSettingKey = 'settings_refresh_button';
const classesLeftUntilSettingKey = 'settings_classes_left_until';
const messTimingsSettingKey = 'settings_mess_timings';
const showSkipAdviceSettingKey = 'settings_show_skip_advice';
const byokGuideDoneSettingKey = 'settings_byok_guide_done';
const forwardingGuideDoneSettingKey = 'settings_forwarding_guide_done';
const vtopCompactModeSettingKey = 'settings_vtop_compact_mode';
const vtopDesktopModeSettingKey = 'settings_vtop_desktop_mode';
const vtopRecentPagesSettingKey = 'settings_vtop_recent_pages';
const _autoRefreshKey = 'settings_auto_refresh';
const _classEnabledKey = 'settings_class_notifications_enabled';
const _classBeforeKey = 'settings_class_notify_before_minutes';
const _classPauseUntilKey = 'settings_class_pause_until_millis';
const _examEnabledKey = 'settings_exam_notifications_enabled';
const _examBeforeKey = 'settings_exam_notify_before_minutes';
const _changeAlertsKey = 'settings_change_alerts_enabled';

final settingsProvider = FutureProvider<SharedPreferencesWithCache>(
  (ref) => SharedPreferencesWithCache.create(
    cacheOptions: SharedPreferencesWithCacheOptions(
      allowList: {
        classesLeftUntilSettingKey,
        messTimingsSettingKey,
        showSkipAdviceSettingKey,
        byokGuideDoneSettingKey,
        forwardingGuideDoneSettingKey,
        _autoRefreshKey,
        refreshButtonSettingKey,
        emailOtpDeleteAfterReadingSettingKey,
        _classEnabledKey,
        _classBeforeKey,
        _classPauseUntilKey,
        _examEnabledKey,
        _examBeforeKey,
        _changeAlertsKey,
        for (final type in ChangeAlertTypeSetting.values) type.prefKey,
        vtopSessionReuseTtlSettingKey,
        'settings_student_projects_pinned_ids',
        'settings_student_projects_json',
        'settings_student_projects_rotation_seed',
        timetableViewModeSettingKey,
        emailOtpBannerDismissedSettingKey,
        vtopCompactModeSettingKey,
        vtopDesktopModeSettingKey,
        'settings_server_url',
        'settings_vtop_proxy',
        'settings_vtop_mode',
        'settings_vtop_udp',
        bridgeServerSignInSettingKey,
        vtopRecentPagesSettingKey,
        bridgeLinkedSettingKey,
        bridgeLastFcmTokenSettingKey,
        bridgeLastSemesterSettingKey,
        onDeviceModeSettingKey,
      },
    ),
  ),
);

/// Writes [value] under [key], or removes it when null. With [legacy] it
/// goes to plain SharedPreferences too, which background work reads.
Future<void> _write(
  Future<SharedPreferencesWithCache> settings,
  String key,
  Object? value, {
  bool legacy = false,
}) async {
  final prefs = await settings;
  final plain = legacy ? await SharedPreferences.getInstance() : null;
  switch (value) {
    case bool value:
      await prefs.setBool(key, value);
      await plain?.setBool(key, value);
    case int value:
      await prefs.setInt(key, value);
      await plain?.setInt(key, value);
    case String value:
      await prefs.setString(key, value);
      await plain?.setString(key, value);
    case null:
      await prefs.remove(key);
      await plain?.remove(key);
  }
}

/// Saves [value] and refreshes [provider], which reads it.
Future<void> _set(
  WidgetRef ref,
  ProviderOrFamily provider,
  String key,
  Object? value, {
  bool legacy = false,
  bool whenMounted = false,
}) async {
  await _write(ref.read(settingsProvider.future), key, value, legacy: legacy);
  if (whenMounted && !ref.context.mounted) return;
  ref.invalidate(provider);
}

Provider<bool> _boolSetting(String key, bool fallback) => Provider.autoDispose(
  (ref) => ref.watch(settingsProvider).value?.getBool(key) ?? fallback,
);

/// How far ahead "classes left" on the attendance cards counts.
enum ClassesLeftUntil {
  /// To the last class before the FAT.
  semesterEnd,

  /// To the next exam: CAT-I, then CAT-II, then the FAT.
  nextExam,
}

final classesLeftUntilProvider = Provider.autoDispose<ClassesLeftUntil>((ref) {
  final stored = ref
      .watch(settingsProvider)
      .value
      ?.getString(classesLeftUntilSettingKey);
  return ClassesLeftUntil.values.firstWhere(
    (value) => value.name == stored,
    orElse: () => ClassesLeftUntil.nextExam,
  );
});

Future<void> setClassesLeftUntil(WidgetRef ref, ClassesLeftUntil value) =>
    _set(ref, classesLeftUntilProvider, classesLeftUntilSettingKey, value.name);

/// Shows skip advice ("Can skip 3", "6 to spare", "need 2") in the
/// timetable and attendance; off by default, when only percentages and
/// counts show.
final showSkipAdviceProvider = _boolSetting(showSkipAdviceSettingKey, false);

Future<void> setShowSkipAdvice(WidgetRef ref, bool value) =>
    _set(ref, showSkipAdviceProvider, showSkipAdviceSettingKey, value);

/// The floating refresh button on the screen edge.
final refreshButtonProvider = _boolSetting(refreshButtonSettingKey, true);

Future<void> setRefreshButton(WidgetRef ref, bool value) =>
    _set(ref, refreshButtonProvider, refreshButtonSettingKey, value);

final autoRefreshProvider = _boolSetting(_autoRefreshKey, true);

Future<bool> isAutoRefreshEnabled(WidgetRef ref) async {
  await ref.read(settingsProvider.future);
  return ref.read(autoRefreshProvider);
}

Future<void> setautoRefresh(WidgetRef ref, bool value) =>
    _set(ref, autoRefreshProvider, _autoRefreshKey, value);

final emailOtpDeleteAfterReadingProvider = _boolSetting(
  emailOtpDeleteAfterReadingSettingKey,
  true,
);

Future<void> setEmailOtpDeleteAfterReading(WidgetRef ref, bool value) => _set(
  ref,
  emailOtpDeleteAfterReadingProvider,
  emailOtpDeleteAfterReadingSettingKey,
  value,
);

final vtopSessionReuseTtlProvider = Provider.autoDispose<Duration>(
  (ref) => vtopSessionReuseTtlFromMinutes(
    ref.watch(settingsProvider).value?.getInt(vtopSessionReuseTtlSettingKey),
  ),
);

Future<void> setVtopSessionReuseTtl(WidgetRef ref, Duration value) => _set(
  ref,
  vtopSessionReuseTtlProvider,
  vtopSessionReuseTtlSettingKey,
  value.inMinutes,
  legacy: true,
);

final classReminderSettingsProvider =
    Provider.autoDispose<
      ({bool enabled, int notifyBeforeMinutes, int? pauseUntilMillis})
    >((ref) {
      final prefs = ref.watch(settingsProvider).value;
      return (
        enabled: prefs?.getBool(_classEnabledKey) ?? false,
        notifyBeforeMinutes: prefs?.getInt(_classBeforeKey) ?? 10,
        pauseUntilMillis: prefs?.getInt(_classPauseUntilKey),
      );
    });

Future<void> setClassReminderEnabled(WidgetRef ref, bool value) async {
  await _write(
    ref.read(settingsProvider.future),
    _classEnabledKey,
    value,
    legacy: true,
  );
  if (!value) await ClassReminderNotificationService.cancelAll();
  if (!ref.context.mounted) return;
  ref.invalidate(classReminderSettingsProvider);
}

Future<void> setClassReminderNotifyBeforeMinutes(WidgetRef ref, int value) =>
    _set(
      ref,
      classReminderSettingsProvider,
      _classBeforeKey,
      value,
      legacy: true,
      whenMounted: true,
    );

Future<void> pauseClassRemindersForDays(WidgetRef ref, int days) async {
  final now = DateTime.now();
  final until = DateTime(now.year, now.month, now.day + days, 23, 59, 59);
  await _write(
    ref.read(settingsProvider.future),
    _classPauseUntilKey,
    until.millisecondsSinceEpoch,
    legacy: true,
  );
  await ClassReminderNotificationService.cancelAll();
  if (!ref.context.mounted) return;
  ref.invalidate(classReminderSettingsProvider);
}

Future<void> clearClassReminderPause(WidgetRef ref) => _set(
  ref,
  classReminderSettingsProvider,
  _classPauseUntilKey,
  null,
  legacy: true,
  whenMounted: true,
);

final examReminderSettingsProvider =
    Provider.autoDispose<({bool enabled, int notifyBeforeMinutes})>((ref) {
      final prefs = ref.watch(settingsProvider).value;
      return (
        enabled: prefs?.getBool(_examEnabledKey) ?? false,
        notifyBeforeMinutes: prefs?.getInt(_examBeforeKey) ?? 10,
      );
    });

Future<void> setExamReminderEnabled(WidgetRef ref, bool value) async {
  await _write(
    ref.read(settingsProvider.future),
    _examEnabledKey,
    value,
    legacy: true,
  );
  if (!value) await ExamReminderNotificationService.cancelAll();
  if (!ref.context.mounted) return;
  ref.invalidate(examReminderSettingsProvider);
}

Future<void> setExamReminderNotifyBeforeMinutes(WidgetRef ref, int value) =>
    _set(
      ref,
      examReminderSettingsProvider,
      _examBeforeKey,
      value,
      legacy: true,
      whenMounted: true,
    );

typedef ChangeAlertTypeSetting = ChangeAlertType;

class ChangeAlertsSettings {
  const ChangeAlertsSettings({required this.enabled, required this.types});

  final bool enabled;
  final Set<ChangeAlertTypeSetting> types;

  bool isEnabled(ChangeAlertTypeSetting type) => types.contains(type);
}

final changeAlertsSettingsProvider = Provider.autoDispose<ChangeAlertsSettings>(
  (ref) {
    final prefs = ref.watch(settingsProvider).value;
    return ChangeAlertsSettings(
      enabled: prefs?.getBool(_changeAlertsKey) ?? true,
      types: {
        for (final type in ChangeAlertTypeSetting.values)
          if (prefs?.getBool(type.prefKey) ?? true) type,
      },
    );
  },
);

final changeAlertsSettingsControllerProvider =
    Provider<ChangeAlertsSettingsController>(
      ChangeAlertsSettingsController.new,
    );

class ChangeAlertsSettingsController {
  ChangeAlertsSettingsController(this.ref);

  final Ref ref;

  Future<void> _save(String key, bool value) async {
    await _write(ref.read(settingsProvider.future), key, value, legacy: true);
    ref.invalidate(changeAlertsSettingsProvider);
  }

  Future<void> setEnabled(bool value) => _save(_changeAlertsKey, value);

  Future<void> setTypeEnabled(ChangeAlertTypeSetting type, bool value) =>
      _save(type.prefKey, value);
}

/// Serialize preference writes while updating the visible selection immediately.
class VtopViewPreference extends Notifier<bool> {
  VtopViewPreference(this.key, this.defaultValue);
  final String key;
  final bool defaultValue;
  Future<void> _writes = Future.value();

  @override
  bool build() =>
      ref.watch(settingsProvider).value?.getBool(key) ?? defaultValue;

  Future<void> setValue(bool value) {
    state = value;
    final write = _writes.then((_) async {
      final prefs = await ref.read(settingsProvider.future);
      await prefs.setBool(key, value);
    });
    // A failed write must not prevent the next selection from being saved.
    _writes = write.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return write;
  }
}

final vtopCompactModeProvider = NotifierProvider<VtopViewPreference, bool>(
  () => VtopViewPreference(vtopCompactModeSettingKey, true),
);
final vtopDesktopModeProvider = NotifierProvider<VtopViewPreference, bool>(
  () => VtopViewPreference(vtopDesktopModeSettingKey, false),
);
