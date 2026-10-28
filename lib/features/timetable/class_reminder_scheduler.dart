import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/platform/notifications/class_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/vtop_page.dart' show Read;

/// Reschedules class reminders from the saved timetable and academic
/// calendar. Reminders with a calendar only cover the next two weeks, so
/// this runs whenever either loads and on every background sync to keep
/// them topped up. Pass [timetable] when calling from the timetable
/// provider itself. Failures are logged, never thrown.
Future<void> rescheduleClassReminders(
  Read read, {
  TimetableData? timetable,
}) async {
  try {
    final data =
        timetable ??
        await read<Future<TimetableData>>(timetableProvider.future);
    SemesterCalendar? calendar;
    try {
      final saved = SemesterCalendar.of(
        await read(academicCalendarProvider.future),
      );
      if (!saved.isEmpty) calendar = saved;
    } catch (_) {
      // No calendar: reminders fall back to repeating weekly.
    }
    await ClassReminderNotificationService.syncFromTimetable(
      data,
      calendar: calendar,
    );
  } catch (e) {
    Log.info('notifications.class_reminders', 'reschedule failed: $e');
  }
}
