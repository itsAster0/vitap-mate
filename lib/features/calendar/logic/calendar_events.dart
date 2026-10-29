import 'package:vitapmate/features/calendar/logic/semester_calendar.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// The most notable mark among a day's entries.
CalendarMark? dayMark(List<CalendarEntry>? entries) {
  if (entries == null || entries.isEmpty) return null;
  return entries.map(markOf).reduce((a, b) => a.index <= b.index ? a : b);
}

/// One row of the month's list: a single day, or a run of listed days with
/// the same title (an exam, the lab FAT). A run is broken only by a class
/// day, so holidays inside it and month ends do not split it.
class CalendarEvent {
  CalendarEvent(this.first, this.entry)
    : last = first,
      parts = [CalendarEventPart(first, entry)];

  final DateTime first;
  DateTime last;

  /// The entry naming the row; for a run of days off, its most telling day.
  CalendarEntry entry;

  bool get isDayOff => _isDayOff(markOf(entry));

  /// Calendar days the row spans, ends included.
  int get days => last.difference(first).inDays + 1;

  /// The reasons inside a run of days off, each over its own days.
  final List<CalendarEventPart> parts;
}

/// One reason inside a run of days off ("Holiday" on Sunday, "Vijaya
/// Dashami / Dussehra" on Tuesday).
class CalendarEventPart {
  CalendarEventPart(this.first, this.entry) : last = first;

  final DateTime first;
  DateTime last;
  final CalendarEntry entry;
}

bool _isDayOff(CalendarMark mark) =>
    mark == CalendarMark.holiday || mark == CalendarMark.noClasses;

/// Which day of a run of days off names it: a named holiday, then a named
/// no-class day ("5th Annual Convocation"), then a plain holiday.
int _dayOffRank(CalendarEntry entry) {
  final holiday = markOf(entry) == CalendarMark.holiday;
  final named = switch (titleOf(entry)) {
    'Holiday' || 'No instructional day' => false,
    _ => true,
  };
  return switch ((named, holiday)) {
    (true, true) => 0,
    (true, false) => 1,
    (false, true) => 2,
    (false, false) => 3,
  };
}

/// Whether classes are held on any day strictly between [a] and [b], so
/// two exam blocks around a class week stay separate rows.
bool _classDayBetween(
  Map<DateTime, List<CalendarEntry>> byDate,
  DateTime a,
  DateTime b,
) {
  for (
    var d = DateTime(a.year, a.month, a.day + 1);
    d.isBefore(b);
    d = DateTime(d.year, d.month, d.day + 1)
  ) {
    final mark = dayMark(byDate[d]);
    if (mark == CalendarMark.classes ||
        mark == CalendarMark.special ||
        mark == CalendarMark.labFat) {
      return true;
    }
  }
  return false;
}

/// The semester's notable days as rows, in date order: holidays, exams,
/// no-class days and named events. Plain class days and Sundays are left
/// out.
List<CalendarEvent> semesterEvents(Map<DateTime, List<CalendarEntry>> byDate) {
  final days = byDate.keys.toList()..sort();
  final events = <CalendarEvent>[];
  for (final day in days) {
    for (final entry in byDate[day]!) {
      final mark = markOf(entry);
      if (mark == CalendarMark.classes) continue;
      // Back-to-back days off (Sunday, a no-class Monday, a festival) are
      // one row, named by its most telling day.
      if (_isDayOff(mark)) {
        final previous = events.reversed.where((e) => e.isDayOff).firstOrNull;
        if (previous != null && day.difference(previous.last).inDays <= 1) {
          previous.last = day;
          final part = previous.parts.last;
          if (titleOf(part.entry) == titleOf(entry) &&
              markOf(part.entry) == mark) {
            part.last = day;
          } else {
            previous.parts.add(CalendarEventPart(day, entry));
          }
          if (_dayOffRank(entry) < _dayOffRank(previous.entry)) {
            previous.entry = entry;
          }
        } else {
          events.add(CalendarEvent(day, entry));
        }
        continue;
      }
      // The latest row of the same kind, even with a holiday row after it.
      final previous = events.reversed
          .where(
            (e) =>
                markOf(e.entry) == mark && titleOf(e.entry) == titleOf(entry),
          )
          .firstOrNull;
      if (previous != null && !_classDayBetween(byDate, previous.last, day)) {
        previous.last = day;
      } else {
        events.add(CalendarEvent(day, entry));
      }
    }
  }
  return events;
}
