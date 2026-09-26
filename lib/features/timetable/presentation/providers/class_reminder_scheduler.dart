import 'package:riverpod_annotation/riverpod_annotation.dart'
    show ProviderListenable;
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/features/calendar/domain/semester_calendar.dart';
import 'package:vitapmate/features/calendar/presentation/providers/academic_calendar_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/services/class_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// Reschedules class reminders from the saved timetable and academic
/// calendar. Reminders with a calendar only cover the next two weeks, so
/// this runs whenever either loads and on every background sync to keep
/// them topped up. Pass [timetable] when calling from the timetable
/// provider itself. Failures are logged, never thrown.
Future<void> rescheduleClassReminders(
  T Function<T>(ProviderListenable<T> provider) read, {
  TimetableData? timetable,
}) async {
  try {
    final TimetableData data =
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
    AppLogger.instance.info(
      'notifications.class_reminders',
      'reschedule failed: $e',
    );
  }
}
