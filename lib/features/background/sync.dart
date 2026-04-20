import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart'
    show ProviderListenable;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/app_errors.dart';
import 'package:vitapmate/core/utils/vtop_login_with_otp.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/full_attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/attendance_repository.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/settings/presentation/providers/state/semester_id.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/state/timetable_repo.dart';
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
  final container = ProviderContainer();
  try {
    await RustLib.init();
    return await syncVtopData(
      read: container.read,
      task: task,
      respectBackgroundFeatureFlag: true,
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
  bool respectBackgroundFeatureFlag = false,
  bool promptForOtp = true,
  bool ignoreRecoverableErrors = false,
}) async {
  try {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('${task}_val_start', DateTime.now().toString());

    final user = await read(vtopUserProvider.future);
    if (!user.isValid || user.semid == null) {
      return true;
    }
    if (respectBackgroundFeatureFlag) {
      final featureFlags = await read(featureFlagsControllerProvider.future);
      if (!await featureFlags.isEnabled("background-sync")) {
        return true;
      }
    }
    await read(
      vClientProvider.notifier,
    ).ensureLogin(promptForOtp: promptForOtp);
    final List<Future<bool>> futures = [];

    final timetable = await (await read(
      timetableRepositoryProvider.future,
    )).load();
    if (force || !_isUpdatedWithinBacksyncWindow(timetable.updateTime)) {
      futures.add(
        _retryer(
          () => read(timetableProvider.notifier).updateTimetable(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }

    final marks = await (await read(marksRepositoryProvider.future)).load();
    if (force || !_isUpdatedWithinBacksyncWindow(marks.updateTime)) {
      futures.add(
        _retryer(
          () => read(marksProvider.notifier).updatemarks(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }

    final examSchedule = await (await read(
      examScheduleRepositoryProvider.future,
    )).load();
    if (force || !_isUpdatedWithinBacksyncWindow(examSchedule.updateTime)) {
      futures.add(
        _retryer(
          () => read(examScheduleProvider.notifier).updatexamschedule(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }

    final semids = await (await read(
      semidRepositoryProvider.future,
    )).getSemidsFromStorage();
    if (force || !_isUpdatedWithinBacksyncWindow(semids.updateTime)) {
      futures.add(
        _retryer(
          () => read(semesterIdProvider.notifier).updatesemids(),
          read: read,
          ignoreRecoverableErrors: ignoreRecoverableErrors,
        ),
      );
    }

    futures.add(
      _attendanceSync(
        read,
        task,
        force: force,
        ignoreRecoverableErrors: ignoreRecoverableErrors,
      ),
    );
    final k = await Future.wait(futures);
    await prefs.setString('${task}_val_end', DateTime.now().toString());
    return atLeastHalfTrue(k);
  } catch (e) {
    if (ignoreRecoverableErrors) {
      return _shouldIgnoreBackgroundError(e, read);
    }
    rethrow;
  }
}

const _backsyncFreshWindow = Duration(minutes: 15);

bool _isUpdatedWithinBacksyncWindow(BigInt updatedAt) {
  final nowUnixSeconds = DateTime.now().millisecondsSinceEpoch ~/ 1000;
  final minFresh = BigInt.from(nowUnixSeconds - _backsyncFreshWindow.inSeconds);
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
      if (e is FeatureDisabledException) {
        return true;
      }
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

Future<bool> _attendanceSync(
  ProviderReader read,
  String? task, {
  required bool force,
  required bool ignoreRecoverableErrors,
}) async {
  try {
    final attendanceRepo = await read(attendanceRepositoryProvider.future);
    var att = await attendanceRepo.load();
    var ok = true;
    if (force || !_isUpdatedWithinBacksyncWindow(att.updateTime)) {
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
          final fullAttendance = await attendanceRepo
              .getFullAttendanceFromStorage(i.courseType, i.courseId);
          if (!force &&
              _isUpdatedWithinBacksyncWindow(fullAttendance.updateTime)) {
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
    return atLeastHalfTrue(k) && ok;
  } catch (e) {
    if (ignoreRecoverableErrors) {
      return _shouldIgnoreBackgroundError(e, read);
    }
    rethrow;
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
