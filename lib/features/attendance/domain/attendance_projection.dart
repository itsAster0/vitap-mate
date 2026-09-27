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
  const AttendanceProjection({
    required this.standing,
    required this.left,
    this.unposted = 0,
  });

  final AttendanceStanding standing;

  /// Classes not yet in [standing]: the [unposted] ones already held, plus
  /// those still to be held this semester.
  final int left;

  /// Of [left], classes already held that VTOP has not posted yet.
  final int unposted;

  /// Of [left], classes still to be held.
  int get upcoming => left - unposted;

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

  /// This projection for a lab in two-period sessions: [standing] as
  /// [AttendanceStanding.sessions], and [left] and [unposted] halved (a lab
  /// day's periods come in pairs).
  AttendanceProjection inSessions(AttendanceRecord record) =>
      AttendanceProjection(
        standing: AttendanceStanding.sessions(record),
        left: (left + 1) ~/ 2,
        unposted: (unposted + 1) ~/ 2,
      );

  /// Counts [record]'s remaining classes: every day from [now] on that the
  /// calendar holds classes for its kind, times its slots on that weekday.
  /// Today's slots count only if they have not started yet. With [until]
  /// (an exam's first day) it stops the day before.
  ///
  /// VTOP posts attendance days late, so the summary usually stops short of
  /// today. With [countedThrough] (the last day the summary covers) before
  /// today, counting starts the day after it instead, so classes held but
  /// not yet posted still count as to come.
  factory AttendanceProjection.of({
    required AttendanceRecord record,
    required TimetableData timetable,
    required SemesterCalendar calendar,
    required DateTime now,
    DateTime? until,
    DateTime? countedThrough,
  }) {
    final (:unposted, :upcoming) = _count(
      code: courseCodeOf(record),
      lab: record.islab(),
      timetable: timetable,
      calendar: calendar,
      now: now,
      until: until,
      countedThrough: countedThrough,
    );
    return AttendanceProjection(
      standing: AttendanceStanding.of(record),
      left: unposted + upcoming,
      unposted: unposted,
    );
  }

  /// The classes still to come for course [code] ([lab] or theory), counted
  /// as in [AttendanceProjection.of].
  static int classesLeft({
    required String code,
    required bool lab,
    required TimetableData timetable,
    required SemesterCalendar calendar,
    required DateTime now,
    DateTime? until,
    DateTime? countedThrough,
  }) {
    final (:unposted, :upcoming) = _count(
      code: code,
      lab: lab,
      timetable: timetable,
      calendar: calendar,
      now: now,
      until: until,
      countedThrough: countedThrough,
    );
    return unposted + upcoming;
  }

  /// Counts held-but-unposted classes (only when [countedThrough] is before
  /// today) and classes still to come.
  static ({int unposted, int upcoming}) _count({
    required String code,
    required bool lab,
    required TimetableData timetable,
    required SemesterCalendar calendar,
    required DateTime now,
    DateTime? until,
    DateTime? countedThrough,
  }) {
    final perDay = <String, List<int>>{};
    for (final slot in timetable.slots) {
      if (slot.courseCode != code || (slot.kind == ClassKind.lab) != lab) {
        continue;
      }
      perDay.putIfAbsent(slot.day, () => []).add(minutesOf(slot.startTime));
    }

    var unposted = 0;
    var upcoming = 0;
    final last = calendar.lastClassDay(lab: lab);
    if (perDay.isEmpty || last == null) return (unposted: 0, upcoming: 0);
    final today = DateTime(now.year, now.month, now.day);
    final lagging = countedThrough != null && countedThrough.isBefore(today);
    final nowMinute = now.hour * 60 + now.minute;
    var day = lagging
        ? DateTime(
            countedThrough.year,
            countedThrough.month,
            countedThrough.day + 1,
          )
        : today;
    while (!day.isAfter(last) && (until == null || day.isBefore(until))) {
      if (calendar.holdsClasses(day, lab: lab)) {
        final starts = perDay[_slotDays[day.weekday - 1]] ?? const [];
        if (day.isBefore(today)) {
          unposted += starts.length;
        } else if (day == today) {
          final ahead = starts.where((start) => start > nowMinute).length;
          upcoming += ahead;
          // Started today but not in the summary: held, unposted.
          if (lagging) unposted += starts.length - ahead;
        } else {
          upcoming += starts.length;
        }
      }
      day = DateTime(day.year, day.month, day.day + 1);
    }
    return (unposted: unposted, upcoming: upcoming);
  }
}
