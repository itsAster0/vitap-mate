import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart'
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

@pragma('vm:entry-point')
void callbackDispatcher() {
  Workmanager().executeTask((task, inputdata) async {
    switch (task) {
      case "sync_vtop":
        await BackgroundNotificationService.initialize();
        await BackgroundNotificationService.showProgress();
        final k = await _syncData(task: task);
        await BackgroundNotificationService.stop(success: k);
        return true;
      default:
        break;
    }
    return Future.value(true);
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
  bool fresh(BigInt updatedAt) => isVtopDataFresh(updatedAt, maxAge);
  try {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('${task}_val_start', DateTime.now().toString());

    final user = await read(vtopUserProvider.future);
    if (!user.isValid || user.semid == null) {
      return true;
    }
    await read(
      vClientProvider.notifier,
    ).ensureLogin(promptForOtp: promptForOtp);
    final List<Future<bool>> futures = [];

    final timetable = await read(timetableProvider.future);
    final refreshTimetable = force || !fresh(timetable.updateTime);
    final marks = await read(marksProvider.future);
    final refreshMarks = force || !fresh(marks.updateTime);
    final examSchedule = await read(examScheduleProvider.future);
    final refreshExamSchedule = force || !fresh(examSchedule.updateTime);
    final semids = await readSavedPage(read, VtopQueries.semesters());
    final refreshSemesters =
        force || semids == null || !fresh(semids.updateTime);

    if (refreshTimetable) {
      futures.add(
        _retryer(
          () => read(timetableProvider.notifier).refresh(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }
    if (refreshMarks) {
      futures.add(
        _retryer(
          () => read(marksProvider.notifier).refresh(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }
    if (refreshExamSchedule) {
      futures.add(
        _retryer(
          () => read(examScheduleProvider.notifier).refresh(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }
    if (refreshSemesters) {
      futures.add(
        _retryer(
          () => read(semesterIdProvider.notifier).refresh(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }

    AttendanceData? previousAttendance;
    futures.add(
      _attendanceSync(
        read,
        task,
        force: force,
        maxAge: maxAge,
        ignoreRecoverableErrors: ignoreRecoverableErrors,
      ).then((result) {
        previousAttendance = result.$2;
        return result.$1;
      }),
    );
    final k = await Future.wait(futures);
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
    return atLeastHalfTrue(k);
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
  return _isFreshSince(updatedAt, minFresh);
}

bool _isFreshSince(BigInt updatedAt, BigInt minFresh) {
  return updatedAt > BigInt.zero && updatedAt >= minFresh;
}

bool atLeastHalfTrue(List<bool> k) {
  if (k.isEmpty) return true;
  final trueCount = k.where((e) => e).length;
  return trueCount * 2 >= k.length;
}

Future<bool> _retryer(
  Future<void> Function() func, {
  required ProviderReader read,
  required bool ignoreRecoverableErrors,
}) async {
  // ignore: non_constant_identifier_names
  final int MAX_TRY = 3;
  int runs = 0;
  while (runs < MAX_TRY) {
    runs += 1;
    try {
      await func();
      return true;
    } catch (e) {
      if (ignoreRecoverableErrors) {
        final shouldIgnore = await _shouldIgnoreBackgroundError(e, read);
        if (!shouldIgnore) return false;
        if (runs == MAX_TRY) return true;
      } else if (runs == MAX_TRY) {
        return false;
      }
      await Future.delayed(Duration(milliseconds: 400 * (runs + 1)));
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
  if (type == AppErrorType.network || type == AppErrorType.sessionExpired) {
    return true;
  }
  return false;
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

    final k = await Future.wait([
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
    return (atLeastHalfTrue(k) && ok, previousAttendance);
  } catch (e) {
    if (ignoreRecoverableErrors) {
      return (await _shouldIgnoreBackgroundError(e, read), null);
    }
    rethrow;
  }
}

const _backgroundSyncTaskName = 'sync_vtop';

Future<void> _runChangeDetection(
  ProviderReader read,
  String? task, {
  TimetableData? previousTimetable,
  MarksData? previousMarks,
  ExamScheduleData? previousExamSchedule,
  AttendanceData? previousAttendance,
}) async {
  if (task != _backgroundSyncTaskName) return;
  try {
    final freshTimetable = await readSavedSemesterPage(
      read,
      VtopQueries.timetable,
    );
    if (freshTimetable != null && previousTimetable != null) {
      final summary = compareTimetable(previousTimetable, freshTimetable);
      if (summary != null) {
        await ChangeAlertNotificationService.showDataChange(
          type: ChangeAlertType.timetable,
          semesterId: freshTimetable.semesterId,
          summary: summary,
        );
      }
    }

    final freshMarks = await readSavedSemesterPage(read, VtopQueries.marks);
    if (freshMarks != null && previousMarks != null) {
      final summary = compareMarks(previousMarks, freshMarks);
      if (summary != null) {
        await ChangeAlertNotificationService.showDataChange(
          type: ChangeAlertType.marks,
          semesterId: freshMarks.semesterId,
          summary: summary,
        );
      }
    }

    final freshExamSchedule = await readSavedSemesterPage(
      read,
      VtopQueries.examSchedule,
    );
    if (freshExamSchedule != null && previousExamSchedule != null) {
      final summary = compareExamSchedule(
        previousExamSchedule,
        freshExamSchedule,
      );
      if (summary != null) {
        await ChangeAlertNotificationService.showDataChange(
          type: ChangeAlertType.examSchedule,
          semesterId: freshExamSchedule.semesterId,
          summary: summary,
        );
      }
    }

    final freshAttendance = await readSavedSemesterPage(
      read,
      VtopQueries.attendance,
    );
    if (freshAttendance != null && previousAttendance != null) {
      final summary = compareAttendance(previousAttendance, freshAttendance);
      if (summary != null) {
        await ChangeAlertNotificationService.showDataChange(
          type: ChangeAlertType.attendance,
          semesterId: freshAttendance.semesterId,
          summary: summary,
        );
      }
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

  static const int _syncId = 9001;
  static const _backSyncId = "background_sync_v2";
  static const _backSyncDoneId = "background_sync_done_v2";
  static const AndroidNotificationChannel _backgroundSyncChannel =
      AndroidNotificationChannel(
        _backSyncId,
        'Background Sync',
        description: 'Background data synchronization',
        importance: Importance.min,
      );
  static const AndroidNotificationChannel _backgroundSyncChannelDone =
      AndroidNotificationChannel(
        _backSyncDoneId,
        'Background Sync done',
        description: 'Background data synchronization done',
        importance: Importance.min,
      );

  static Future<void> initialize() async {
    const AndroidInitializationSettings androidSettings =
        AndroidInitializationSettings('@mipmap/launcher_icon');

    const settings = InitializationSettings(android: androidSettings);

    await _notifications.initialize(settings: settings);

    await _notifications
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >()
        ?.createNotificationChannel(_backgroundSyncChannelDone);

    await _notifications
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >()
        ?.createNotificationChannel(_backgroundSyncChannel);
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

  static Future<void> stop({required bool success}) async {
    await _notifications.cancel(id: _syncId);

    // await _notifications.show(
    //   _syncId + 1,
    //   'VITAP Mate',
    //   success ? 'Background sync completed' : 'Failed to sync some data',
    //   const NotificationDetails(
    //     android: AndroidNotificationDetails(
    //       _backSyncDoneId,
    //       'Background Sync done',
    //       importance: Importance.min,
    //       priority: Priority.min,
    //       silent: true,
    //     ),
    //   ),
    // );
  }
}
