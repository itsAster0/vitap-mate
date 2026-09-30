import 'dart:developer';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/attendance/domain/attendance_history.dart';
import 'package:vitapmate/features/attendance/presentation/providers/full_attendance_provider.dart';
import 'package:vitapmate/features/calendar/presentation/providers/academic_calendar_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'attendance_provider.g.dart';

@Riverpod(keepAlive: true)
class Attendance extends _$Attendance {
  Future<AttendanceData> _runLoad() async {
    final repo = await ref.watch(attendanceRepositoryProvider.future);
    final controller = VtopController<AttendanceData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-attendance",
    );
    return controller.load();
  }

  @override
  Future<AttendanceData> build() async {
    final attendance = await _runLoad();
    log("attendance build done");
    return attendance;
  }

  Future<void> updateAttendance() async {
    final repo = await ref.read(attendanceRepositoryProvider.future);
    final controller = VtopController<AttendanceData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-attendance",
    );
    final attendance = await controller.refresh();
    state = AsyncData(attendance);
    // The projection needs a current calendar, but it rarely changes, so
    // it is refetched only when a day old. Not awaited, and failures only
    // log, so attendance never waits on it.
    ref
        .read(academicCalendarProvider.notifier)
        .refreshIfStale(calendarMaxAgeOnAttendance);
  }

  /// Pulls the class history of each course whose saved history no longer
  /// matches its summary. VTOP keeps the two in sync, so a mismatch means
  /// the saved history is behind. Courses never opened have no history yet
  /// and are left alone. One course at a time; a failure only logs.
  Future<void> pullMismatchedHistory() async {
    for (final record in state.value?.records ?? const <AttendanceRecord>[]) {
      final history = await ref.read(
        cachedFullAttendanceProvider(record.courseType, record.courseId).future,
      );
      if (history == null || HistorySync.of(record, history).inSync) continue;
      try {
        await ref
            .read(
              fullAttendanceProvider(
                record.courseType,
                record.courseId,
              ).notifier,
            )
            .updateAttendance();
      } catch (e, st) {
        log('history pull failed for ${record.courseCode}: $e', stackTrace: st);
      }
    }
  }
}
