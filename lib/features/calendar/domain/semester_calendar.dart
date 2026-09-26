import 'package:vitapmate/src/api/vtop/types.dart';

const _instructionalDay = 'instructional day';
const _holiday = 'holiday';
const _labFat = 'lab fat';

/// The academic calendar reduced to what attendance projection needs:
/// which days hold classes, and when lab classes stop for the lab FAT.
class SemesterCalendar {
  SemesterCalendar._({
    required Set<DateTime> instructionalDays,
    required this.labFatStart,
    required List<({DateTime date, String name})> holidays,
    required Map<DateTime, CalendarEntry> dayEntries,
    required List<({DateTime start, String name})> exams,
  }) : _instructionalDays = instructionalDays,
       _exams = exams,
       _holidays = holidays,
       _dayEntries = dayEntries,
       _firstDay = dayEntries.keys.fold<DateTime?>(
         null,
         (a, b) => a == null || b.isBefore(a) ? b : a,
       ),
       _lastDay = dayEntries.keys.fold<DateTime?>(
         null,
         (a, b) => a == null || b.isAfter(a) ? b : a,
       );

  /// Uses the "General (Semester)" events when the calendar has any, since
  /// the combined view can also list other class groups' days.
  factory SemesterCalendar.of(AcademicCalendarData data) {
    final entries = generalEntries(data);

    final instructional = <DateTime>{};
    final holidays = <({DateTime date, String name})>[];
    final dayEntries = <DateTime, CalendarEntry>{};
    final examStarts = <String, DateTime>{};
    DateTime? labFatStart;
    for (final entry in entries) {
      final date = DateTime.tryParse(entry.date);
      if (date == null) continue;
      final day = _dateOnly(date);
      final existing = dayEntries[day];
      if (existing == null || markOf(entry).index < markOf(existing).index) {
        dayEntries[day] = entry;
      }
      final kind = entry.kind.toLowerCase();
      if (markOf(entry) == CalendarMark.exam) {
        final start = examStarts[entry.kind];
        if (start == null || day.isBefore(start)) examStarts[entry.kind] = day;
      }
      if (kind == _instructionalDay) {
        instructional.add(day);
        if (entry.note.toLowerCase().contains(_labFat) &&
            (labFatStart == null || day.isBefore(labFatStart))) {
          labFatStart = day;
        }
      } else if (kind == _holiday && day.weekday != DateTime.sunday) {
        holidays.add((date: day, name: entry.note));
      }
    }
    holidays.sort((a, b) => a.date.compareTo(b.date));
    return SemesterCalendar._(
      instructionalDays: instructional,
      labFatStart: labFatStart,
      holidays: holidays,
      dayEntries: dayEntries,
      exams: [
        for (final MapEntry(key: kind, value: start) in examStarts.entries)
          (start: start, name: _shortExamName(kind)),
      ]..sort((a, b) => a.start.compareTo(b.start)),
    );
  }

  final Set<DateTime> _instructionalDays;
  final List<({DateTime date, String name})> _holidays;

  /// Each exam (CAT - I, CAT - II, FAT) by its first day, in date order.
  final List<({DateTime start, String name})> _exams;

  /// The next exam with classes still to come before it: CAT-I, then
  /// CAT-II, then the FAT. An exam under way, or one with no class day left
  /// before it (the weekend before a CAT), counts as passed. Null once no
  /// class days remain.
  ({DateTime start, String name})? nextExam(DateTime now) {
    final today = _dateOnly(now);
    DateTime? nextClassDay;
    for (final day in _instructionalDays) {
      if (day.isBefore(today)) continue;
      if (nextClassDay == null || day.isBefore(nextClassDay)) {
        nextClassDay = day;
      }
    }
    if (nextClassDay == null) return null;
    for (final exam in _exams) {
      if (exam.start.isAfter(nextClassDay)) return exam;
    }
    return null;
  }

  /// The most notable entry of each dated day.
  final Map<DateTime, CalendarEntry> _dayEntries;

  /// First and last listed day.
  final DateTime? _firstDay, _lastDay;

  /// First lab FAT day. Lab classes are not held from this day on; theory
  /// classes carry on as usual.
  final DateTime? labFatStart;

  bool get isEmpty => _instructionalDays.isEmpty;

  /// Whether [day] falls between the first and last listed day. Outside
  /// that the calendar says nothing, so callers fall back to the plain
  /// timetable.
  bool covers(DateTime day) {
    final first = _firstDay, last = _lastDay;
    if (first == null || last == null) return false;
    final date = _dateOnly(day);
    return !date.isBefore(first) && !date.isAfter(last);
  }

  /// The day's most notable entry (exam over holiday over class day), or
  /// null for a day the calendar does not list.
  CalendarEntry? entryOn(DateTime day) => _dayEntries[_dateOnly(day)];

  /// Whether classes of this kind meet on [day]: per the calendar where it
  /// lists the day, otherwise assumed (so the timetable still works outside
  /// the semester's months).
  bool classesOn(DateTime day, {required bool lab}) =>
      !covers(day) || holdsClasses(day, lab: lab);

  /// Whether classes of this kind are held on [day].
  bool holdsClasses(DateTime day, {required bool lab}) {
    final date = _dateOnly(day);
    if (!_instructionalDays.contains(date)) return false;
    final stop = labFatStart;
    return !lab || stop == null || date.isBefore(stop);
  }

  /// Instructional days after today.
  int instructionalDaysLeft(DateTime now) {
    final today = _dateOnly(now);
    return _instructionalDays.where((d) => d.isAfter(today)).length;
  }

  /// The next weekday holiday from today on (Sundays are always off).
  ({DateTime date, String name})? nextHoliday(DateTime now) {
    final today = _dateOnly(now);
    for (final holiday in _holidays) {
      if (!holiday.date.isBefore(today)) return holiday;
    }
    return null;
  }

  /// The last day with classes of this kind, or null for an empty calendar.
  DateTime? lastClassDay({required bool lab}) {
    DateTime? last;
    for (final day in _instructionalDays) {
      if (!holdsClasses(day, lab: lab)) continue;
      if (last == null || day.isAfter(last)) last = day;
    }
    return last;
  }
}

DateTime _dateOnly(DateTime date) => DateTime(date.year, date.month, date.day);

/// "CAT - II" → "CAT-II", "Final Assessment Test" → "FAT".
String _shortExamName(String kind) {
  if (kind.toLowerCase() == 'final assessment test') return 'FAT';
  return kind.replaceAll(' - ', '-');
}

/// The "General (Semester)" events when the calendar has any, since the
/// combined view can also list other class groups' days.
List<CalendarEntry> generalEntries(AcademicCalendarData data) {
  final general = data.entries
      .where((e) => e.group.toLowerCase().startsWith('general'))
      .toList();
  return general.isEmpty ? data.entries : general;
}

/// How a calendar day is marked, most notable first.
enum CalendarMark { exam, labFat, holiday, noClasses, special, classes }

/// The mark for one entry. "Special" is a class day with its own note
/// ("First Instructional Day"); a plain working day is [CalendarMark.classes].
CalendarMark markOf(CalendarEntry entry) {
  final kind = entry.kind.toLowerCase();
  final note = entry.note.toLowerCase();
  if (kind == _instructionalDay) {
    if (note.contains(_labFat)) return CalendarMark.labFat;
    if (note.isEmpty || note == 'workingday') return CalendarMark.classes;
    return CalendarMark.special;
  }
  if (kind == _holiday) return CalendarMark.holiday;
  if (kind.startsWith('no instructional')) return CalendarMark.noClasses;
  return CalendarMark.exam;
}

/// What the day is called: the holiday or event name, else the exam or day
/// type ("CAT - II", "No instructional day").
String titleOf(CalendarEntry entry) {
  final note = entry.note.trim();
  return switch (markOf(entry)) {
    CalendarMark.holiday => note.isEmpty ? 'Holiday' : note,
    CalendarMark.exam => entry.kind,
    CalendarMark.labFat => 'Lab FAT',
    CalendarMark.noClasses =>
      note.isEmpty || note.toLowerCase() == 'no instructional day'
          ? 'No instructional day'
          : note,
    CalendarMark.special ||
    CalendarMark.classes => note.isEmpty ? entry.kind : note,
  };
}
