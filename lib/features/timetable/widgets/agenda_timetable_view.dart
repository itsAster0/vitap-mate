import 'dart:math' as math;
import 'dart:async';

import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/ui/extensions.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/features/exams/exams.dart';
import 'package:vitapmate/features/timetable/logic/time_format.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'agenda/sections.dart';
part 'agenda/header.dart';
part 'agenda/focus.dart';
part 'agenda/class_row.dart';

enum AgendaClassStatus { completed, current, next, upcoming }

/// Day view of the timetable: a live "now / next" panel on top of the day's
/// schedule, with attendance guidance on every class.
class AgendaTimetableView extends HookWidget {
  const AgendaTimetableView({
    super.key,
    required this.selectedDay,
    required this.classDays,
    required this.slotsForDay,
    this.attendance = const [],
    this.calendar,
    this.exams = const [],
    this.history = const {},
  });

  /// ISO weekday (1 = Monday) being shown.
  final ValueNotifier<int> selectedDay;
  final Set<int> classDays;

  /// A weekday's timetable, before the academic calendar is applied.
  final List<TimetableSlot> Function(int weekday) slotsForDay;
  final List<AttendanceRecord> attendance;

  /// Drops classes on holidays and exam days, and explains the day off.
  final SemesterCalendar? calendar;

  /// Every paper in the exam schedule, for the days without classes.
  final List<ExamPaper> exams;

  /// Each course's saved class history by (course code, lab), for the
  /// week recap on days without classes.
  final Map<(String, bool), FullAttendanceData> history;

  @override
  Widget build(BuildContext context) {
    final now = useState(DateTime.now());
    useEffect(() {
      // Status changes on minute boundaries; sync the first tick to one.
      Timer? periodic;
      final first = Timer(Duration(seconds: 60 - DateTime.now().second), () {
        now.value = DateTime.now();
        periodic = Timer.periodic(
          const Duration(minutes: 1),
          (_) => now.value = DateTime.now(),
        );
      });
      return () {
        first.cancel();
        periodic?.cancel();
      };
    }, const []);

    final weekDates = _weekDates(now.value);
    final week = [for (var d = 1; d <= 7; d++) ...slotsForDay(d)];
    ClassDay dayOf(TimetableSlot slot, DateTime date) =>
        classDayOf(slot, date, calendar, week);
    List<TimetableSlot> slotsOn(DateTime date) => [
      for (final slot in slotsForDay(date.weekday))
        if (dayOf(slot, date) != ClassDay.off) slot,
    ];
    final today = DateTime(now.value.year, now.value.month, now.value.day);
    // The day's regular schedule is always listed; [slots] are the classes
    // that actually meet, which drive the focus card, day bar and statuses.
    final schedule = slotsForDay(selectedDay.value);
    final selectedDate = weekDates[selectedDay.value - 1];
    final kinds = {for (final s in schedule) s: dayOf(s, selectedDate)};
    final slots = [
      for (final s in schedule)
        if (kinds[s] != ClassDay.off) s,
    ];
    final labFats = {
      for (final s in schedule)
        if (kinds[s] == ClassDay.labFat) s,
    };
    final nextDay = _nextClassDay(selectedDate, slotsOn);
    // What the calendar says about the day: a holiday or exam that cancels
    // it, or a note (Lab FAT, an event) on a day that still has classes.
    final dayEntry = calendar?.entryOn(selectedDate);
    final dayMark = dayEntry == null ? null : markOf(dayEntry);
    // A class day where only labs are off (after the Lab FAT) is no day off.
    final dayOff =
        dayEntry != null &&
            slots.isEmpty &&
            schedule.isNotEmpty &&
            !(calendar?.holdsClasses(selectedDate, lab: false) ?? false)
        ? dayEntry
        : null;
    final lastClassDay = calendar?.lastClassDay(lab: false);
    // Plain "Holiday" Sundays and "No instructional day" Mondays say
    // nothing new, so only named days get a note. Lab FAT only when you
    // write one.
    final dayNote = slots.isNotEmpty && selectedDate == lastClassDay
        ? 'Last class day'
        : dayEntry == null ||
              dayOff != null ||
              dayMark == CalendarMark.classes ||
              (dayMark == CalendarMark.labFat && labFats.isEmpty) ||
              const {
                'Holiday',
                'No instructional day',
              }.contains(titleOf(dayEntry))
        ? null
        : titleOf(dayEntry);
    final isToday = selectedDate == today;
    final past = selectedDate.isBefore(today);
    final minuteNow = _timeOfDay(now.value);

    final current = isToday
        ? slots
              .where(
                (s) =>
                    minutesOf(s.startTime) <= minuteNow &&
                    minutesOf(s.endTime) > minuteNow,
              )
              .firstOrNull
        : null;
    final next = past
        ? null
        : slots
              .where((s) => !isToday || minutesOf(s.startTime) > minuteNow)
              .firstOrNull;
    final remaining = isToday
        ? slots.where((s) => minutesOf(s.endTime) > minuteNow).length
        : slots.length;
    // Today after the last class, or an earlier day: the card recaps it.
    final dayDone = slots.isNotEmpty && current == null && next == null;

    // The date each course meets for the last time this semester.
    final lastDates = calendar == null
        ? const <(String, bool), DateTime>{}
        : _lastClassDates(week, calendar!, dayOf);
    bool isLastClass(TimetableSlot s) =>
        kinds[s] == ClassDay.held &&
        lastDates[(s.courseCode, s.kind == ClassKind.lab)] == selectedDate;

    // Real breaks (VIT leaves 10 minutes between classes); the longest one is
    // tagged when the day has more than one.
    final gaps = [
      for (var i = 1; i < schedule.length; i++)
        minutesOf(schedule[i].startTime) - minutesOf(schedule[i - 1].endTime),
    ];
    final realGaps = gaps.where((g) => g >= _minBreak).toList();
    final longestGap = realGaps.length > 1
        ? realGaps.reduce((a, b) => a > b ? a : b)
        : null;

    // Once the day is over, each class's posted attendance, in order.
    final statuses = dayDone
        ? _dayStatuses(slots, selectedDate, labFats)
        : null;
    final posted = {for (final c in statuses ?? const []) c.slot: c.status};

    Widget classRow(int i) {
      final slot = schedule[i];
      final kind = kinds[slot]!;
      // Not meeting (a holiday, or a lab after the Lab FAT): listed as
      // usual, with no live state.
      final off = kind == ClassDay.off;
      final live = isToday && !off;
      return EnterFade(
        index: i,
        child: _ClassRow(
          slot: slot,
          status: _statusFor(slot, current, next, minuteNow, live),
          // A Lab FAT doesn't count towards attendance.
          record: kind == ClassDay.labFat
              ? null
              : attendanceForSlot(attendance, slot),
          minuteNow: live ? minuteNow : null,
          isFirst: i == 0,
          isLast: i == schedule.length - 1,
          labFat: kind == ClassDay.labFat,
          lastClass: isLastClass(slot),
          // On a class day, a lab that doesn't meet says so; a day off
          // says it once, in the card.
          note: off && dayOff == null && slot.kind == ClassKind.lab
              ? 'No lab'
              : null,
          showPosted: posted.containsKey(slot) && kind != ClassDay.labFat,
          posted: posted[slot],
        ),
      );
    }

    // Previous day index tells the switcher which way to slide.
    final previousDay = usePrevious(selectedDay.value) ?? selectedDay.value;
    final forward = selectedDay.value >= previousDay;

    return Padding(
      padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, 0),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          _DateHeading(
            date: selectedDate,
            isToday: isToday,
            slots: slots,
            note: dayOff == null ? dayNote : titleOf(dayOff),
            onToday: () => selectedDay.value = now.value.weekday,
          ),
          if (slots.isNotEmpty) ...[
            const SizedBox(height: Space.md),
            _DayBar(slots: slots, minuteNow: isToday ? minuteNow : null),
          ],
          const SizedBox(height: Space.md),
          _WeekStrip(
            dates: weekDates,
            today: now.value.weekday,
            selectedDay: selectedDay,
            classCounts: [for (final d in weekDates) slotsOn(d).length],
            examDays: {
              for (final p in exams)
                for (final (i, date) in weekDates.indexed)
                  if (_sameDay(p.start, date)) i + 1,
            },
          ),
          const SizedBox(height: Space.md),
          AnimatedSwitcher(
            duration: Motion.slow,
            // Old day fades out in the first third, new day fades in after,
            // so the two never overlap.
            switchInCurve: const Interval(0.35, 1, curve: Curves.easeOutCubic),
            switchOutCurve: const Interval(0.65, 1, curve: Curves.easeIn),
            transitionBuilder: (child, animation) {
              final incoming = child.key == ValueKey(selectedDay.value);
              final dx = (incoming == forward ? 1 : -1) * 0.04;
              return FadeTransition(
                opacity: animation,
                child: SlideTransition(
                  position: Tween(
                    begin: Offset(dx, 0),
                    end: Offset.zero,
                  ).animate(animation),
                  child: child,
                ),
              );
            },
            layoutBuilder: (currentChild, previous) => Stack(
              alignment: .topCenter,
              children: [...previous, ?currentChild],
            ),
            child: Column(
              key: ValueKey(selectedDay.value),
              crossAxisAlignment: .stretch,
              children: [
                _FocusPanel(
                  current: current,
                  next: next,
                  now: now.value,
                  isToday: isToday,
                  slots: slots,
                  attendance: attendance,
                  labFats: labFats,
                  isLastClass: isLastClass,
                  done: statuses,
                  // A day without your classes says what the calendar calls
                  // it (holiday, exam, no classes), even if it never has any.
                  dayOff:
                      dayOff ??
                      (slots.isEmpty &&
                              dayMark != null &&
                              dayMark != CalendarMark.classes &&
                              dayMark != CalendarMark.special &&
                              dayMark != CalendarMark.labFat
                          ? dayEntry
                          : null),
                  calendarListed: dayEntry != null,
                  papers: [
                    for (final p in exams)
                      if (_sameDay(p.start, selectedDate)) p,
                  ],
                  // A paper before the next class is what's next.
                  nextPaper: _papersBetween(
                    exams,
                    from: DateTime(
                      selectedDate.year,
                      selectedDate.month,
                      selectedDate.day + 1,
                    ),
                    now: now.value,
                    before: nextDay?.date,
                  ).firstOrNull,
                  upcomingDay: slots.isEmpty ? nextDay : null,
                  countdown: slots.isEmpty
                      ? _countdown(selectedDate, lastClassDay, slotsOn)
                      : null,
                ),
                // A day your timetable has no classes on looks at the week
                // ahead and the courses to watch.
                if (schedule.isEmpty) ...[
                  _WeekAhead(
                    from: selectedDate,
                    calendar: calendar,
                    papers: exams,
                    slotsOn: slotsOn,
                    labFatsOn: (date) => [
                      for (final s in slotsForDay(date.weekday))
                        if (dayOf(s, date) == ClassDay.labFat) s,
                    ],
                    onOpen: (date) => weekDates.contains(date)
                        ? () => selectedDay.value = date.weekday
                        : null,
                  ),
                  _AttendanceWatch(records: attendance),
                ],
                if (schedule.isNotEmpty)
                  SectionHeader(
                    title: isToday ? 'Today' : 'Schedule',
                    trailing: Text(
                      slots.isEmpty
                          ? switch (dayMark) {
                              CalendarMark.holiday => 'Holiday',
                              CalendarMark.exam => 'Exam day',
                              _ => 'No classes',
                            }
                          : isToday && remaining < slots.length
                          ? '$remaining of ${slots.length} left'
                          : '${slots.length} ${slots.length == 1 ? 'class' : 'classes'}',
                    ),
                  ),
                for (var i = 0; i < schedule.length; i++) ...[
                  if (i > 0)
                    _BreakRow(
                      previousEnd: schedule[i - 1].endTime,
                      nextStart: schedule[i].startTime,
                      isNow:
                          isToday &&
                          dayOff == null &&
                          minutesOf(schedule[i - 1].endTime) <= minuteNow &&
                          minutesOf(schedule[i].startTime) > minuteNow,
                      minuteNow: minuteNow,
                      isLongest: gaps[i - 1] == longestGap,
                    ),
                  classRow(i),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }

  /// Each class of [slots] on [date], in order, with its posted status from
  /// the saved history: null when not posted yet or a Lab FAT.
  List<({TimetableSlot slot, ClassStatus? status})> _dayStatuses(
    List<TimetableSlot> slots,
    DateTime date,
    Set<TimetableSlot> labFats,
  ) {
    final taken = <(String, bool), int>{};
    return [
      for (final slot in slots)
        if (labFats.contains(slot))
          (slot: slot, status: null)
        else
          () {
            final key = (slot.courseCode, slot.kind == ClassKind.lab);
            final rows = [
              for (final row in history[key]?.records ?? const [])
                if (parseHistoryDate(row.date) == date)
                  classStatusOf(row.status),
            ];
            final i = taken[key] = (taken[key] ?? -1) + 1;
            return (slot: slot, status: i < rows.length ? rows[i] : null);
          }(),
    ];
  }

  /// "12 class days left · FAT starts Mon 16 Nov", counting your days with
  /// classes after [from]; null without a calendar or anything to say.
  String? _countdown(
    DateTime from,
    DateTime? lastClassDay,
    List<TimetableSlot> Function(DateTime date) slotsOn,
  ) {
    final exam = calendar?.upcomingExam(from);
    var left = 0;
    if (lastClassDay != null) {
      for (
        var d = DateTime(from.year, from.month, from.day + 1);
        !d.isAfter(lastClassDay);
        d = DateTime(d.year, d.month, d.day + 1)
      ) {
        if (slotsOn(d).isNotEmpty) left++;
      }
    }
    final line = [
      if (left > 0) '$left class ${left == 1 ? 'day' : 'days'} left',
      if (exam != null)
        '${exam.name} starts ${DateFormat('EEE d MMM').format(exam.start)}',
    ].join(' · ');
    return line.isEmpty ? null : line;
  }

  /// The next day after [from] that has classes, with its first class. With
  /// a calendar it looks further ahead, past exam weeks and breaks.
  ({DateTime date, TimetableSlot first})? _nextClassDay(
    DateTime from,
    List<TimetableSlot> Function(DateTime date) slotsOn,
  ) {
    final horizon = calendar == null ? 7 : 28;
    for (var offset = 1; offset <= horizon; offset++) {
      final date = DateTime(from.year, from.month, from.day + offset);
      final slots = slotsOn(date);
      if (slots.isNotEmpty) return (date: date, first: slots.first);
    }
    return null;
  }
}

/// The last date each course (code, lab) meets this semester, looking back
/// from the last class day.
Map<(String, bool), DateTime> _lastClassDates(
  List<TimetableSlot> week,
  SemesterCalendar calendar,
  ClassDay Function(TimetableSlot slot, DateTime date) dayOf,
) {
  final last = calendar.lastClassDay(lab: false);
  final first = calendar.firstClassDay;
  if (last == null || first == null) return const {};
  final courses = {
    for (final s in week) (s.courseCode, s.kind == ClassKind.lab),
  };
  final found = <(String, bool), DateTime>{};
  for (
    var d = last;
    !d.isBefore(first) && found.length < courses.length;
    d = DateTime(d.year, d.month, d.day - 1)
  ) {
    for (final s in week) {
      if (slotDays.indexOf(s.day.trim().toUpperCase()) + 1 != d.weekday) {
        continue;
      }
      if (dayOf(s, d) == ClassDay.held) {
        found.putIfAbsent((s.courseCode, s.kind == ClassKind.lab), () => d);
      }
    }
  }
  return found;
}

AgendaClassStatus _statusFor(
  TimetableSlot slot,
  TimetableSlot? current,
  TimetableSlot? next,
  int minuteNow,
  bool isToday,
) {
  if (slot == current) return AgendaClassStatus.current;
  if (slot == next && isToday) return AgendaClassStatus.next;
  if (isToday && minutesOf(slot.endTime) <= minuteNow) {
    return AgendaClassStatus.completed;
  }
  return AgendaClassStatus.upcoming;
}

/// Whole days from [from] to [date].
int _inDays(DateTime date, DateTime from) =>
    DateTime(date.year, date.month, date.day).difference(from).inDays;

bool _sameDay(DateTime a, DateTime b) =>
    a.year == b.year && a.month == b.month && a.day == b.day;

/// Papers still to be written, from [from] (a day) up to [before], in time
/// order.
List<ExamPaper> _papersBetween(
  List<ExamPaper> exams, {
  required DateTime from,
  required DateTime now,
  DateTime? before,
}) => [
  for (final p in exams)
    if (p.start.isAfter(now) &&
        !p.start.isBefore(from) &&
        (before == null || p.start.isBefore(before)))
      p,
];

/// "Tomorrow", a weekday name within the coming week, else "Tue 6 Oct".
String _dayLabel(DateTime date, DateTime now) {
  final today = DateTime(now.year, now.month, now.day);
  final days = _inDays(date, today);
  if (days == 1) return 'Tomorrow';
  if (days > 1 && days < 7) return DateFormat('EEEE').format(date);
  return DateFormat('EEE d MMM').format(date);
}

DateTime _at(DateTime day, String time) {
  final minutes = minutesOf(time);
  return DateTime(day.year, day.month, day.day, minutes ~/ 60, minutes % 60);
}

/// The date of each weekday this week, Monday first.
List<DateTime> _weekDates(DateTime now) => List.generate(
  7,
  (i) => DateTime(now.year, now.month, now.day + 1 - now.weekday + i),
);

int _timeOfDay(DateTime value) => value.hour * 60 + value.minute;
