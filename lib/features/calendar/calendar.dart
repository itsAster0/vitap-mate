/// The academic calendar for the selected semester.
library;

import 'dart:async';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/calendar/logic/semester_calendar.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

export 'package:vitapmate/features/calendar/logic/semester_calendar.dart';

/// How old the saved calendar may get before an attendance refresh fetches
/// it again (catching holidays announced mid-semester).
const calendarMaxAgeOnAttendance = Duration(hours: 24);

final academicCalendarProvider =
    AsyncNotifierProvider<AcademicCalendar, AcademicCalendarData>(
      AcademicCalendar.new,
    );

class AcademicCalendar extends VtopPage<AcademicCalendarData> {
  AcademicCalendar() : super(VtopQueries.academicCalendar);

  @override
  Future<void> refresh() async {
    await super.refresh();
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
      Log.info(
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
final semesterCalendarProvider = Provider<SemesterCalendar?>((ref) {
  final data = ref.watch(academicCalendarProvider).value;
  if (data == null) return null;
  final calendar = SemesterCalendar.of(data);
  return calendar.isEmpty ? null : calendar;
});
