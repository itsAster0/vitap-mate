/// VTOP's rules for hostel outing applications.
///
/// The limits (offered hours, slots, places, how far ahead, allowed weekdays,
/// text lengths) come from the outing pages themselves, via
/// [GeneralOutingData] and [WeekendOutingData]. What VTOP only checks in its
/// page script lives here as constants: the 24-hour notice for a general
/// outing and the return-after-leaving check.
library;

import 'package:intl/intl.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// VTOP's general outing page refuses a leaving time less than this far off.
const generalOutingNotice = Duration(hours: 24);

/// What the outing date fields post (jQuery UI `dd-M-yy`): `11-Oct-2026`.
String vtopOutingDate(DateTime date) =>
    DateFormat('dd-MMM-yyyy', 'en_US').format(date);

DateTime dateOnly(DateTime moment) =>
    DateTime(moment.year, moment.month, moment.day);

/// A time of day in minutes since midnight.
typedef DayMinutes = int;

DayMinutes dayMinutes(int hour, int minute) => hour * 60 + minute;

/// The earliest and latest times a general outing form offers for [hours]:
/// whole hours from the list, the last one only at :00.
({DayMinutes earliest, DayMinutes latest})? offeredWindow(List<int> hours) =>
    hours.isEmpty
    ? null
    : (earliest: dayMinutes(hours.first, 0), latest: dayMinutes(hours.last, 0));

bool isOfferedTime(List<int> hours, int hour, int minute) {
  if (hours.isEmpty) return true;
  if (!hours.contains(hour)) return false;
  return hour != hours.last || minute == 0;
}

/// `6:00 AM`.
String clockLabel(DayMinutes minutes) => DateFormat(
  'h:mm a',
  'en_US',
).format(DateTime(2000, 1, 1, minutes ~/ 60, minutes % 60));

/// `6 AM`, `6:30 PM`: [clockLabel] without a zero minute.
String shortClock(DayMinutes minutes) => DateFormat(
  minutes % 60 == 0 ? 'h a' : 'h:mm a',
  'en_US',
).format(DateTime(2000, 1, 1, minutes ~/ 60, minutes % 60));

/// Leaving dates a general outing may use: today up to `maxDaysAhead`.
({DateTime first, DateTime last}) generalLeavingRange(
  GeneralOutingData form,
  DateTime now,
) {
  final today = dateOnly(now);
  return (first: today, last: today.add(Duration(days: form.maxDaysAhead)));
}

/// How far after leaving the return date picker goes: a year. VTOP's own
/// longest trip (`maxDaysAway`, 15 days) is shown but not enforced.
const generalReturnPickerDays = 365;

/// Return dates the picker offers for an outing leaving on [leaving].
({DateTime first, DateTime last}) generalReturnRange(DateTime leaving) {
  final day = dateOnly(leaving);
  return (
    first: day,
    last: day.add(const Duration(days: generalReturnPickerDays)),
  );
}

/// The problems with a general outing, by field. Empty when VTOP would
/// accept it.
class GeneralOutingCheck {
  const GeneralOutingCheck({
    this.place,
    this.purpose,
    this.leaving,
    this.returning,
  });

  final String? place;
  final String? purpose;
  final String? leaving;
  final String? returning;

  bool get isValid =>
      place == null && purpose == null && leaving == null && returning == null;
}

GeneralOutingCheck checkGeneralOuting({
  required GeneralOutingData form,
  required DateTime now,
  required String place,
  required String purpose,
  required DateTime? leaving,
  required DateTime? returning,
}) {
  final leavingRange = generalLeavingRange(form, now);
  final outWindow = offeredWindow(form.outHours);
  final inWindow = offeredWindow(form.inHours);

  String? leavingError;
  if (leaving == null) {
    leavingError = 'Pick when you leave';
  } else if (dateOnly(leaving).isAfter(leavingRange.last)) {
    leavingError =
        'Up to ${form.maxDaysAhead} days ahead '
        '(${DateFormat('d MMM', 'en_US').format(leavingRange.last)})';
  } else if (!isOfferedTime(form.outHours, leaving.hour, leaving.minute)) {
    leavingError =
        'Leave between ${clockLabel(outWindow!.earliest)} '
        'and ${clockLabel(outWindow.latest)}';
  } else if (leaving.difference(now) < generalOutingNotice) {
    leavingError =
        'Needs 24 h notice: leave after '
        '${DateFormat('EEE d MMM, h:mm a', 'en_US').format(now.add(generalOutingNotice))}';
  }

  String? returningError;
  if (returning == null) {
    returningError = 'Pick when you are back';
  } else if (!isOfferedTime(form.inHours, returning.hour, returning.minute)) {
    returningError =
        'Be back between ${clockLabel(inWindow!.earliest)} '
        'and ${clockLabel(inWindow.latest)}';
  } else if (leaving != null && !returning.isAfter(leaving)) {
    returningError = 'Has to be after you leave';
  }

  return GeneralOutingCheck(
    place: _textError(place, form.placeMaxLength, "Add where you're going"),
    purpose: _textError(purpose, form.purposeMaxLength, "Add why you're going"),
    leaving: leavingError,
    returning: returningError,
  );
}

/// Collapses runs of whitespace, as VTOP's own form effectively does.
String tidyText(String value) => value.trim().split(RegExp(r'\s+')).join(' ');

String? _textError(String value, int maxLength, String missing) {
  final tidy = tidyText(value);
  if (tidy.isEmpty) return missing;
  if (tidy.length > maxLength) return 'At most $maxLength characters';
  return null;
}

/// The days weekend outings are usually on (0 = Sunday, 1 = Monday).
const usualWeekendOutingDays = [0, 1];

/// The weekdays a weekend outing can be on (VTOP numbers them like
/// JavaScript, 0 = Sunday). VTOP's picker also blocks the day before an
/// outing day, so on Saturday it leaves only Monday; tomorrow is added back
/// when it is one of [usualWeekendOutingDays].
List<int> weekendOutingDays(WeekendOutingData form, DateTime now) {
  final tomorrow = dateOnly(now).add(const Duration(days: 1)).weekday % 7;
  return {
    ...form.weekdays,
    if (usualWeekendOutingDays.contains(tomorrow)) tomorrow,
  }.toList()..sort();
}

/// Dates a weekend outing can be applied for: tomorrow up to
/// `maxDaysAhead`, on [weekendOutingDays]. Never today: an outing cannot be
/// applied for on its own day, though Saturday up to midnight is fine for
/// Sunday. VTOP's own picker allows today, so this is the app's rule, not
/// parsed.
List<DateTime> weekendOutingDates(WeekendOutingData form, DateTime now) {
  final today = dateOnly(now);
  final weekdays = weekendOutingDays(form, now);
  return [
    for (var i = 1; i <= form.maxDaysAhead; i++)
      if (weekdays.contains(today.add(Duration(days: i)).weekday % 7))
        today.add(Duration(days: i)),
  ];
}

/// "Sundays and Mondays" for the allowed weekdays.
String weekdayNames(List<int> weekdays) => _listDays(weekdays, 'and');

/// "Sun & Mon" for the allowed weekdays.
String weekdayShortNames(List<int> weekdays) =>
    _listDays(weekdays, '&', short: true);

String _listDays(List<int> weekdays, String and, {bool short = false}) {
  const names = [
    'Sundays',
    'Mondays',
    'Tuesdays',
    'Wednesdays',
    'Thursdays',
    'Fridays',
    'Saturdays',
  ];
  final listed = [
    for (final day in weekdays)
      if (day < 7) short ? names[day].substring(0, 3) : names[day],
  ];
  if (listed.length <= 1) return listed.join();
  return '${listed.sublist(0, listed.length - 1).join(', ')} $and ${listed.last}';
}

class WeekendOutingCheck {
  const WeekendOutingCheck({this.purpose, this.date, this.contact});

  final String? purpose;
  final String? date;
  final String? contact;

  bool get isValid => purpose == null && date == null && contact == null;
}

WeekendOutingCheck checkWeekendOuting({
  required WeekendOutingData form,
  required DateTime now,
  required String purpose,
  required DateTime? date,
  required String contact,
}) {
  final digits = contact.trim();
  return WeekendOutingCheck(
    purpose: _textError(purpose, form.purposeMaxLength, "Add why you're going"),
    date: switch (date) {
      null => 'Pick a day',
      _ when weekendOutingDates(form, now).contains(dateOnly(date)) => null,
      _ => 'Not a day VTOP offers',
    },
    contact: RegExp(r'^\d{10}$').hasMatch(digits)
        ? null
        : 'A 10-digit mobile number',
  );
}

enum OutingStatus { pending, approved, rejected, other }

OutingStatus outingStatus(String status) => switch (status.toLowerCase()) {
  final t when t.contains('accepted') || t.contains('approved') => .approved,
  final t when t.contains('waiting') || t.contains('pending') => .pending,
  final t
      when t.contains('reject') ||
          t.contains('declin') ||
          t.contains('cancel') =>
    .rejected,
  _ => .other,
};

/// The 24-hour hour of a `9:30 PM` match: groups hour, minute, AM/PM.
int _hour24(RegExpMatch m) =>
    int.parse(m[1]!) % 12 + (m[3]!.toUpperCase() == 'PM' ? 12 : 0);

/// Parses VTOP's `YYYY-MM-DD` with an optional `hh:mm AM` time.
DateTime? parseOutingMoment(String date, [String time = '']) {
  final day = DateTime.tryParse(date.trim());
  if (day == null) return null;
  final match = RegExp(
    r'^(\d{1,2}):(\d{2})\s*([AP]M)$',
    caseSensitive: false,
  ).firstMatch(time.trim());
  if (match == null) return dateOnly(day);
  return DateTime(
    day.year,
    day.month,
    day.day,
    _hour24(match),
    int.parse(match[2]!),
  );
}

/// The leaving and return of a weekend slot like `9:30 AM- 3:30PM`.
({DayMinutes start, DayMinutes end})? parseTimeSlot(String slot) {
  final times = RegExp(
    r'(\d{1,2}):(\d{2})\s*([AP]M)',
    caseSensitive: false,
  ).allMatches(slot).toList();
  if (times.length != 2) return null;
  DayMinutes read(RegExpMatch m) => dayMinutes(_hour24(m), int.parse(m[2]!));
  return (start: read(times[0]), end: read(times[1]));
}

/// `9:30 AM – 3:30 PM` for VTOP's `9:30 AM- 3:30PM`.
String slotLabel(String slot) => switch (parseTimeSlot(slot)) {
  final t? => '${clockLabel(t.start)} – ${clockLabel(t.end)}',
  null => slot,
};

/// How long a general outing lasts, for its card: calendar days for a trip
/// across days ("4 days"), hours for one inside a day ("8 h 30 min"). Null
/// for a same-day trip without a return time.
String? awayLabel(DateTime start, DateTime end, {required bool hasReturnTime}) {
  final days = dateOnly(end).difference(dateOnly(start)).inDays;
  if (days > 0) return days == 1 ? '1 day' : '$days days';
  if (!hasReturnTime || !end.isAfter(start)) return null;
  final span = end.difference(start);
  final hours = span.inHours;
  final minutes = span.inMinutes % 60;
  return [if (hours > 0) '$hours h', if (minutes > 0) '$minutes min'].join(' ');
}
