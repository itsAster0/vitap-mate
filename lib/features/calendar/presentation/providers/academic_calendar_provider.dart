import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';
import 'package:vitapmate/features/calendar/data/academic_calendar_repository.dart';
import 'package:vitapmate/features/calendar/domain/semester_calendar.dart';
import 'package:vitapmate/features/timetable/presentation/providers/class_reminder_scheduler.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'academic_calendar_provider.g.dart';

/// How old the saved calendar may get before an attendance refresh fetches
/// it again (catching holidays announced mid-semester).
const calendarMaxAgeOnAttendance = Duration(hours: 24);

@Riverpod(keepAlive: true)
Future<AcademicCalendarRepository> academicCalendarRepository(Ref ref) async {
  return AcademicCalendarRepository(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    dataSource: AcademicCalendarDataSource(
      await ref.read(jsonFileStorageProvider.future),
      () => ref.read(vtopBackendProvider),
      ref.read(globalAsyncQueueProvider.notifier),
    ),
  );
}

@Riverpod(keepAlive: true)
class AcademicCalendar extends _$AcademicCalendar {
  Future<VtopController<AcademicCalendarData>> _controller({
    bool watch = true,
  }) async {
    final repo = academicCalendarRepositoryProvider.future;
    return VtopController<AcademicCalendarData>(
      ref: ref,
      repository: await (watch ? ref.watch(repo) : ref.read(repo)),
      featureName: 'fetch-academic-calendar',
    );
  }

  /// The saved calendar; VTOP is only asked when there is none.
  @override
  Future<AcademicCalendarData> build() async => (await _controller()).load();

  /// Refetches the calendar now. Errors reach the caller.
  Future<void> refresh() async {
    state = AsyncData(await (await _controller(watch: false)).refresh());
    // A holiday may have been added: move reminders off it.
    unawaited(rescheduleClassReminders(ref.read));
  }

  /// Refetches the calendar when the saved copy is older than [maxAge].
  /// Failures are logged; the saved copy stays in use.
  Future<void> refreshIfStale(Duration maxAge) async {
    AcademicCalendarData? current;
    try {
      // Waits for the saved copy if the provider is still loading it.
      current = await future;
    } catch (_) {
      // No saved copy and the first fetch failed: try again below.
    }
    if (current != null && !_isStale(current, maxAge)) return;
    try {
      await refresh();
    } catch (e) {
      AppLogger.instance.info(
        'client.fetch-academic-calendar',
        'background refresh failed: $e',
      );
    }
  }

  static bool _isStale(AcademicCalendarData data, Duration maxAge) {
    final fetched = DateTime.fromMillisecondsSinceEpoch(
      data.updateTime.toInt() * 1000,
    );
    return data.entries.isEmpty || DateTime.now().difference(fetched) > maxAge;
  }
}

/// The saved calendar in the shape attendance projection uses, or null
/// while it is loading, failed to load, or has no instructional days.
@Riverpod(keepAlive: true)
SemesterCalendar? semesterCalendar(Ref ref) {
  final data = ref.watch(academicCalendarProvider).value;
  if (data == null) return null;
  final calendar = SemesterCalendar.of(data);
  return calendar.isEmpty ? null : calendar;
}
