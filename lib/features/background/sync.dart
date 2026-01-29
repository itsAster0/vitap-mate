import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/full_attendance_provider.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/frb_generated.dart';
import 'package:workmanager/workmanager.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';

@pragma('vm:entry-point')
void callbackDispatcher() {
  Workmanager().executeTask((task, inputdata) async {
    switch (task) {
      case "sync_vtop":
        await BackgroundNotificationService.initialize();
        await BackgroundNotificationService.showProgress();
        final k = await _syncData(task: task);
        await BackgroundNotificationService.stop(success: k);
        return k;
      default:
        break;
    }
    return Future.value(true);
  });
}

Future<bool> _syncData({String? task}) async {
  final container = ProviderContainer();
  try {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('${task}_val_start', DateTime.now().toString());

    await RustLib.init();
    final user = await container.read(vtopUserProvider.future);
    if (!user.isValid || user.semid == null) {
      return true;
    }
    var gb = await container.read(gbProvider.future);
    var feature = gb.feature("background-sync");
    if (!(feature.on && feature.value)) {
      return true;
    }
    final List<Future<bool>> futures = [
      _retryer(
        () => container.read(timetableProvider.notifier).updateTimetable(),
      ),

      _retryer(() => container.read(marksProvider.notifier).updatemarks()),

      _retryer(
        () => container.read(examScheduleProvider.notifier).updatexamschedule(),
      ),

      _retryer(
        () => container.read(semesterIdProvider.notifier).updatesemids(),
      ),

      _attendanceSync(container, task),
    ];
    final k = await Future.wait(futures);
    await prefs.setString('${task}_val_end', DateTime.now().toString());
    return k.every((e) => e);
  } catch (e) {
    return false;
  } finally {
    container.dispose();
  }
}

Future<bool> _retryer(Future<void> Function() func) async {
  // ignore: non_constant_identifier_names
  final int MAX_TRY = 4;
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
    }
  }
  return false;
}

Future<bool> _attendanceSync(ProviderContainer container, String? task) async {
  try {
    final ok = await _retryer(
      () => container.read(attendanceProvider.notifier).updateAttendance(),
    );
    final att = await container.read(attendanceProvider.future);
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('${task}_val', att.toString());

    final k = await Future.wait([
      for (final i in att.records)
        _retryer(
          () =>
              container
                  .read(
                    fullAttendanceProvider(i.courseType, i.courseId).notifier,
                  )
                  .updateAttendance(),
        ),
    ]);
    return k.every((e) => e) && ok;
  } catch (e) {
    return false;
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

    await _notifications.initialize(settings);

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
      _syncId,
      'VITAP Mate',
      'Syncing VTOP data in background…',
      const NotificationDetails(
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
    await _notifications.cancel(_syncId);

    await _notifications.show(
      _syncId + 1,
      'VITAP Mate',
      success ? 'Background sync completed' : 'Failed to sync some data',
      const NotificationDetails(
        android: AndroidNotificationDetails(
          _backSyncDoneId,
          'Background Sync done',
          importance: Importance.min,
          priority: Priority.min,
          silent: true,
        ),
      ),
    );
  }
}
