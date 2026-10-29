/// Attendance: the per-course summary and each course's class history.
library;

import 'package:vitapmate/observability/log.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/attendance/logic/attendance_history.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

export 'package:vitapmate/features/attendance/logic/attendance_history.dart';
export 'package:vitapmate/features/attendance/logic/attendance_standing.dart';
export 'package:vitapmate/features/attendance/widgets/attendance.dart';
export 'package:vitapmate/features/attendance/widgets/course_card.dart'
    show Dot;

final attendanceProvider = AsyncNotifierProvider<Attendance, AttendanceData>(
  Attendance.new,
);

class Attendance extends VtopPage<AttendanceData> {
  Attendance() : super(VtopQueries.attendance);

  Future<void> updateAttendance() async {
    await refresh();
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
      final history = await readSavedFullAttendance(
        ref.read,
        record.courseType,
        record.courseId,
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
        Log.warn(
          'features.attendance',
          'history pull failed for ${record.courseCode}: $e\n$st',
        );
      }
    }
  }
}

typedef AttendanceCourse = ({String courseType, String courseId});

VtopQuery<FullAttendanceData> Function(String) _history(
  AttendanceCourse course,
) =>
    (semester) => VtopQueries.fullAttendance(
      semester,
      courseId: course.courseId,
      courseType: course.courseType,
    );

/// One course's class history.
AsyncNotifierProvider<FullAttendance, FullAttendanceData>
fullAttendanceProvider(String courseType, String courseId) =>
    _fullAttendance((courseType: courseType, courseId: courseId));

final _fullAttendance =
    AsyncNotifierProvider.family<
      FullAttendance,
      FullAttendanceData,
      AttendanceCourse
    >(FullAttendance.new);

class FullAttendance extends VtopPage<FullAttendanceData> {
  FullAttendance(this.course) : super(_history(course));

  final AttendanceCourse course;

  Future<void> updateAttendance() async {
    await refresh();
    ref.invalidate(
      cachedFullAttendanceProvider(course.courseType, course.courseId),
    );
  }
}

/// The course's history as last saved, or null if it was never fetched.
/// Never goes to VTOP, so the attendance list can read every course's
/// history without a request each.
FutureProvider<FullAttendanceData?> cachedFullAttendanceProvider(
  String courseType,
  String courseId,
) => _cachedFullAttendance((courseType: courseType, courseId: courseId));

final _cachedFullAttendance = FutureProvider.family(
  (ref, AttendanceCourse course) =>
      readSavedFullAttendance(ref.watch, course.courseType, course.courseId),
);

Future<FullAttendanceData?> readSavedFullAttendance(
  Read read,
  String courseType,
  String courseId,
) => readSavedSemesterPage(
  read,
  _history((courseType: courseType, courseId: courseId)),
);
