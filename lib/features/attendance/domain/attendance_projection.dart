import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/calendar/domain/semester_calendar.dart';
import 'package:vitapmate/features/timetable/presentation/utils/time_format.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

const _slotDays = ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN'];

/// Where a course ends up by the last class of the semester, counting the
/// classes still to come from the timetable and academic calendar.
///
/// Best effort: it cannot know about holidays announced later, cancelled or
/// extra classes, or attendance VTOP has not posted yet.
class AttendanceProjection {
  const AttendanceProjection({required this.standing, required this.left});

  final AttendanceStanding standing;

  /// Classes still to be held this semester.
  final int left;

  int get _finalTotal => standing.total + left;

  /// Classes that can be missed from here and still finish at or above 75%
  /// (same strict arithmetic as [AttendanceStanding.canSkip]).
  int get canMiss {
    final n = ((4 * (standing.attended + left) - 3 * _finalTotal) / 4).floor();
    return n.clamp(0, left);
  }

  /// Classes that must be attended to finish at 75%, or null when even
  /// attending all of them is not enough.
  int? get mustAttend {
    final n = ((3 * _finalTotal - 4 * standing.attended) / 4).ceil();
    if (n > left) return null;
    return n < 0 ? 0 : n;
  }

  /// The percentage when every remaining class is attended.
  double get bestPercent =>
      _finalTotal == 0 ? 0 : (standing.attended + left) / _finalTotal * 100;

  bool get canFinishSafe => mustAttend != null;

  /// Counts [record]'s remaining classes: every day from [now] on that the
  /// calendar holds classes for its kind, times its slots on that weekday.
  /// Today's slots count only if they have not started yet. With [until]
  /// (an exam's first day) it stops the day before.
  factory AttendanceProjection.of({
    required AttendanceRecord record,
    required TimetableData timetable,
    required SemesterCalendar calendar,
    required DateTime now,
    DateTime? until,
  }) {
    final code = courseCodeOf(record);
    final lab = record.islab();
    final perDay = <String, List<int>>{};
    for (final slot in timetable.slots) {
      if (slot.courseCode != code || (slot.kind == ClassKind.lab) != lab) {
        continue;
      }
      perDay.putIfAbsent(slot.day, () => []).add(minutesOf(slot.startTime));
    }

    var left = 0;
    final last = calendar.lastClassDay(lab: lab);
    if (perDay.isNotEmpty && last != null) {
      final nowMinute = now.hour * 60 + now.minute;
      var day = DateTime(now.year, now.month, now.day);
      final today = day;
      while (!day.isAfter(last) && (until == null || day.isBefore(until))) {
        if (calendar.holdsClasses(day, lab: lab)) {
          final starts = perDay[_slotDays[day.weekday - 1]] ?? const [];
          left += day == today
              ? starts.where((start) => start > nowMinute).length
              : starts.length;
        }
        day = DateTime(day.year, day.month, day.day + 1);
      }
    }
    return AttendanceProjection(
      standing: AttendanceStanding.of(record),
      left: left,
    );
  }
}
