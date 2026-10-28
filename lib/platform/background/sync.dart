import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:hooks_riverpod/misc.dart'
    show ProviderListenable;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/vtop/clinet_provider.dart';
import 'package:vitapmate/vtop/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/observability/app_logger.dart';
import 'package:vitapmate/vtop/app_errors.dart';
import 'package:vitapmate/vtop/vtop_login_with_otp.dart';
import 'package:vitapmate/platform/background/change_detection/change_detector.dart';
import 'package:vitapmate/features/exams/exams.dart';
import 'package:vitapmate/features/marks/marks.dart';
import 'package:vitapmate/vtop/semesters.dart';
import 'package:vitapmate/platform/notifications/change_alert_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/frb_generated.dart';
import 'package:workmanager/workmanager.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';

typedef ProviderReader = T Function<T>(ProviderListenable<T> provider);

/// The WorkManager task that runs the periodic background sync.
const backgroundSyncTask = 'sync_vtop';

@pragma('vm:entry-point')
void callbackDispatcher() {
  Workmanager().executeTask((task, inputdata) async {
    if (task == backgroundSyncTask) {
      await BackgroundNotificationService.initialize();
      await BackgroundNotificationService.showProgress();
      final success = await _syncData(task: task);
      await BackgroundNotificationService.stop(success: success);
    }
    return true;
  });
}

Future<bool> _syncData({String? task}) async {
  // Headless: nobody can answer an OTP prompt here.
  final container = ProviderContainer(
    overrides: [vtopLoginPromptAllowedProvider.overrideWithValue(false)],
  );
  try {
    await RustLib.init();
    return await syncVtopData(
      read: container.read,
      task: task,
      promptForOtp: false,
      ignoreRecoverableErrors: true,
    );
  } finally {
    container.dispose();
  }
}

Future<bool> syncVtopData({
  required ProviderReader read,
  String? task,
  bool force = false,
  bool promptForOtp = true,
  bool ignoreRecoverableErrors = false,
  Duration maxAge = backgroundSyncMaxAge,
}) async {
  bool stale(BigInt updatedAt) => force || !isVtopDataFresh(updatedAt, maxAge);
  Future<bool> retry(Future<void> Function() refresh) => _retryer(
    refresh,
    read: read,
    ignoreRecoverableErrors: ignoreRecoverableErrors,
  );
  try {
    // When each run started and ended, kept for debugging.
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('${task}_val_start', DateTime.now().toString());

    final user = await read(vtopUserProvider.future);
    if (!user.isValid || user.semid == null) return true;
    await read(
      vClientProvider.notifier,
    ).ensureLogin(promptForOtp: promptForOtp);

    final timetable = await read(timetableProvider.future);
    final marks = await read(marksProvider.future);
    final examSchedule = await read(examScheduleProvider.future);
    final semids = await readSavedPage(read, VtopQueries.semesters());
    final futures = [
      if (stale(timetable.updateTime))
        retry(() => read(timetableProvider.notifier).refresh()),
      if (stale(marks.updateTime))
        retry(() => read(marksProvider.notifier).refresh()),
      if (stale(examSchedule.updateTime))
        retry(() => read(examScheduleProvider.notifier).refresh()),
      if (semids == null || stale(semids.updateTime))
        retry(() => read(semesterIdProvider.notifier).refresh()),
    ];

    final attendance = _attendanceSync(
      read,
      task,
      force: force,
      maxAge: maxAge,
      ignoreRecoverableErrors: ignoreRecoverableErrors,
    );
    final results = await Future.wait([
      ...futures,
      attendance.then((result) => result.$1),
    ]);
    final (_, previousAttendance) = await attendance;
    // Dated class reminders cover two weeks; keep them topped up even when
    // the app isn't opened.
    await rescheduleClassReminders(read);
    await _runChangeDetection(
      read,
      task,
      previousTimetable: timetable,
      previousMarks: marks,
      previousExamSchedule: examSchedule,
      previousAttendance: previousAttendance,
    );
    await prefs.setString('${task}_val_end', DateTime.now().toString());
    return _atLeastHalfTrue(results);
  } catch (e) {
    if (ignoreRecoverableErrors) {
      return _shouldIgnoreBackgroundError(e, read);
    }
    rethrow;
  }
}

/// Periodic background sync refetches anything older than this.
const backgroundSyncMaxAge = Duration(minutes: 15);

/// Whether data saved at [updatedAt] (unix seconds) is younger than [maxAge].
bool isVtopDataFresh(BigInt updatedAt, Duration maxAge) {
  final nowUnixSeconds = DateTime.now().millisecondsSinceEpoch ~/ 1000;
  final minFresh = BigInt.from(nowUnixSeconds - maxAge.inSeconds);
  return updatedAt > BigInt.zero && updatedAt >= minFresh;
}

bool _atLeastHalfTrue(List<bool> results) =>
    results.where((ok) => ok).length * 2 >= results.length;

const _maxTries = 3;

/// Runs [func] up to [_maxTries] times. In the background, errors that a
/// later run can recover from count as success once the tries run out.
Future<bool> _retryer(
  Future<void> Function() func, {
  required ProviderReader read,
  required bool ignoreRecoverableErrors,
}) async {
  for (var run = 1; run <= _maxTries; run++) {
    try {
      await func();
      return true;
    } catch (e) {
      if (ignoreRecoverableErrors) {
        if (!await _shouldIgnoreBackgroundError(e, read)) return false;
        if (run == _maxTries) return true;
      } else if (run == _maxTries) {
        return false;
      }
      await Future.delayed(Duration(milliseconds: 400 * (run + 1)));
    }
  }
  return false;
}

Future<bool> _shouldIgnoreBackgroundError(
  Object error,
  ProviderReader read,
) async {
  if (isSecurityOtpRequiredError(error)) {
    return read(vtopOtpChallengeProvider.notifier).canAutoFetchFromEmail();
  }
  final (type, _) = appError(error);
  return type == AppErrorType.network || type == AppErrorType.sessionExpired;
}

Future<(bool, AttendanceData?)> _attendanceSync(
  ProviderReader read,
  String? task, {
  required bool force,
  required Duration maxAge,
  required bool ignoreRecoverableErrors,
}) async {
  bool fresh(BigInt updatedAt) => isVtopDataFresh(updatedAt, maxAge);
  try {
    var att = await read(attendanceProvider.future);
    final previousAttendance = att;
    var ok = true;
    if (force || !fresh(att.updateTime)) {
      ok = await _retryer(
        () => read(attendanceProvider.notifier).updateAttendance(),
        read: read,
        ignoreRecoverableErrors: ignoreRecoverableErrors,
      );
      try {
        att = await read(attendanceProvider.future);
      } catch (e) {
        if (!ignoreRecoverableErrors) rethrow;
        if (!await _shouldIgnoreBackgroundError(e, read)) rethrow;
      }
    }
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('${task}_val', att.toString());

    final results = await Future.wait([
      for (final i in att.records)
        () async {
          final fullAttendance = await readSavedFullAttendance(
            read,
            i.courseType,
            i.courseId,
          );
          // One VTOP request per course: skip those just fetched or already
          // matching the summary.
          if (!force &&
              fullAttendance != null &&
              (fresh(fullAttendance.updateTime) ||
                  !historyNeedsRefresh(i, fullAttendance))) {
            return true;
          }
          return _retryer(
            () => read(
              fullAttendanceProvider(i.courseType, i.courseId).notifier,
            ).updateAttendance(),
            read: read,
            ignoreRecoverableErrors: ignoreRecoverableErrors,
          );
        }(),
    ]);
    return (_atLeastHalfTrue(results) && ok, previousAttendance);
  } catch (e) {
    if (ignoreRecoverableErrors) {
      return (await _shouldIgnoreBackgroundError(e, read), null);
    }
    rethrow;
  }
}

Future<void> _runChangeDetection(
  ProviderReader read,
  String? task, {
  TimetableData? previousTimetable,
  MarksData? previousMarks,
  ExamScheduleData? previousExamSchedule,
  AttendanceData? previousAttendance,
}) async {
  if (task != backgroundSyncTask) return;
  Future<void> alert(
    ChangeAlertType type,
    String semesterId,
    DataChangeSummary? summary,
  ) async {
    if (summary == null) return;
    await ChangeAlertNotificationService.showDataChange(
      type: type,
      semesterId: semesterId,
      summary: summary,
    );
  }

  try {
    if (await readSavedSemesterPage(read, VtopQueries.timetable)
        case final now?) {
      await alert(
        .timetable,
        now.semesterId,
        compareTimetable(previousTimetable, now),
      );
    }
    if (await readSavedSemesterPage(read, VtopQueries.marks) case final now?) {
      await alert(.marks, now.semesterId, compareMarks(previousMarks, now));
    }
    if (await readSavedSemesterPage(read, VtopQueries.examSchedule)
        case final now?) {
      await alert(
        .examSchedule,
        now.semesterId,
        compareExamSchedule(previousExamSchedule, now),
      );
    }
    if (await readSavedSemesterPage(read, VtopQueries.attendance)
        case final now?) {
      await alert(
        .attendance,
        now.semesterId,
        compareAttendance(previousAttendance, now),
      );
    }
  } catch (e) {
    AppLogger.instance.warning(
      'background.change_detection',
      'Change detection failed: $e',
    );
  }
}

class BackgroundNotificationService {
  static final FlutterLocalNotificationsPlugin _notifications =
      FlutterLocalNotificationsPlugin();

  static const _syncId = 9001;
  static const _backSyncId = "background_sync_v2";
  static const _backgroundSyncChannel = AndroidNotificationChannel(
    _backSyncId,
    'Background Sync',
    description: 'Background data synchronization',
    importance: Importance.min,
  );
  static const _backgroundSyncChannelDone = AndroidNotificationChannel(
    "background_sync_done_v2",
    'Background Sync done',
    description: 'Background data synchronization done',
    importance: Importance.min,
  );

  static Future<void> initialize() async {
    await _notifications.initialize(
      settings: const InitializationSettings(
        android: AndroidInitializationSettings('@mipmap/launcher_icon'),
      ),
    );
    final android = _notifications
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >();
    await android?.createNotificationChannel(_backgroundSyncChannelDone);
    await android?.createNotificationChannel(_backgroundSyncChannel);
  }

  static Future<void> showProgress() async {
    await _notifications.show(
      id: _syncId,
      title: 'VITAP Mate',
      body: 'Syncing VTOP data in background…',
      notificationDetails: const NotificationDetails(
        android: AndroidNotificationDetails(
          _backSyncId,
          'Background Sync',
          importance: Importance.low,
          priority: Priority.low,
          ongoing: true,
          indeterminate: true,
          showProgress: true,
          silent: true,
          timeoutAfter: 1000 * 60 * 5,
        ),
      ),
    );
  }

  /// Clears the progress notification; [success] is not shown for now.
  static Future<void> stop({required bool success}) =>
      _notifications.cancel(id: _syncId);
}
