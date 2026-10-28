import 'dart:math' as math;
import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/app/settings.dart';
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
    List<TimetableSlot> daySlots(int weekday) =>
        classesOnDate(slotsForDay(weekday), weekDates[weekday - 1], calendar);
    // The day's regular schedule is always listed; [slots] are the classes
    // that actually meet, which drive the hero card, day bar and statuses.
    final schedule = slotsForDay(selectedDay.value);
    final slots = daySlots(selectedDay.value);
    final selectedDate = weekDates[selectedDay.value - 1];
    // What the calendar says about the day: a holiday or exam that cancels
    // it, or a note (Lab FAT, an event) on a day that still has classes.
    final dayEntry = calendar?.entryOn(selectedDate);
    final dayMark = dayEntry == null ? null : markOf(dayEntry);
    final dayOff = dayEntry != null && slots.isEmpty && schedule.isNotEmpty
        ? dayEntry
        : null;
    // Plain "Holiday" Sundays and "No instructional day" Mondays say
    // nothing new, so only named days get a note.
    final dayNote =
        dayEntry == null ||
            dayOff != null ||
            dayMark == CalendarMark.classes ||
            const {
              'Holiday',
              'No instructional day',
            }.contains(titleOf(dayEntry))
        ? null
        : titleOf(dayEntry);
    final isToday = selectedDay.value == now.value.weekday;
    final today = DateTime(now.value.year, now.value.month, now.value.day);
    final minuteNow = _timeOfDay(now.value);

    final current = isToday
        ? _firstWhereOrNull(
            slots,
            (s) =>
                minutesOf(s.startTime) <= minuteNow &&
                minutesOf(s.endTime) > minuteNow,
          )
        : null;
    final next = _firstWhereOrNull(
      slots,
      (s) => !isToday || minutesOf(s.startTime) > minuteNow,
    );
    final remaining = isToday
        ? slots.where((s) => minutesOf(s.endTime) > minuteNow).length
        : slots.length;

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

    // Previous day index tells the switcher which way to slide.
    final previousDay = usePrevious(selectedDay.value) ?? selectedDay.value;
    final forward = selectedDay.value >= previousDay;

    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.sm, Space.sm, Space.sm, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
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
            classCounts: [for (var d = 1; d <= 7; d++) daySlots(d).length],
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
              alignment: Alignment.topCenter,
              children: [...previous, ?currentChild],
            ),
            child: Column(
              key: ValueKey(selectedDay.value),
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _FocusPanel(
                  current: current,
                  next: next,
                  now: now.value,
                  isToday: isToday,
                  slots: slots,
                  attendance: attendance,
                  // A day without your classes says what the calendar calls
                  // it (holiday, exam, no classes), even if it never has any.
                  dayOff:
                      dayOff ??
                      (slots.isEmpty &&
                              dayMark != null &&
                              dayMark != CalendarMark.classes &&
                              dayMark != CalendarMark.special
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
                    before: _nextClassDay(selectedDate)?.date,
                  ).firstOrNull,
                  upcomingDay: slots.isEmpty || (isToday && next == null)
                      ? _nextClassDay(selectedDate)
                      : null,
                ),
                // A day without classes: fill it with what comes next.
                if (slots.isEmpty && !selectedDate.isBefore(today))
                  _ComingUp(
                    from: selectedDate,
                    now: now.value,
                    nextDay: _nextClassDay(selectedDate),
                    slotsOn: (date) => classesOnDate(
                      slotsForDay(date.weekday),
                      date,
                      calendar,
                    ),
                    exams: exams,
                    attendance: attendance,
                    calendar: calendar,
                  ),
                // A day your timetable has no classes on at all recaps
                // attendance: a Sunday the week it ends, any other day last
                // week. A class day called off (a holiday) shows its dimmed
                // schedule instead.
                if (schedule.isEmpty)
                  _WeekRecap(
                    thisWeek: selectedDate.weekday == DateTime.sunday,
                    dates: [
                      for (final d in weekDates)
                        selectedDate.weekday == DateTime.sunday
                            ? d
                            : DateTime(d.year, d.month, d.day - 7),
                    ],
                    now: now.value,
                    classesOn: (date) => classesOnDate(
                      slotsForDay(date.weekday),
                      date,
                      calendar,
                    ),
                    history: history,
                  ),
                // Sundays look ahead at exams and holidays.
                if (selectedDate.weekday == DateTime.sunday && calendar != null)
                  _Lookahead(
                    from: selectedDate,
                    calendar: calendar!,
                    papers: exams,
                    // Exams whose papers "Coming up" already lists.
                    hide: {
                      if (!selectedDate.isBefore(today))
                        for (final p in _papersBetween(
                          exams,
                          from: selectedDate,
                          now: now.value,
                          before: _nextClassDay(selectedDate)?.date,
                        ))
                          p.name,
                    },
                  ),
                if (schedule.isNotEmpty)
                  SectionHeader(
                    title: isToday ? 'Today' : 'Schedule',
                    trailing: Text(
                      slots.isEmpty
                          ? 'No classes'
                          : isToday && remaining < slots.length
                          ? '$remaining of ${slots.length} left'
                          : '${slots.length} ${slots.length == 1 ? 'class' : 'classes'}',
                    ),
                  ),
                for (var i = 0; i < schedule.length; i++) ...[
                  if (i > 0)
                    _dimmedIf(
                      dayOff != null,
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
                    ),
                  Builder(
                    builder: (context) {
                      // Called off by the calendar (a holiday, or a lab
                      // after the LAB FAT): shown dimmed, with no live state.
                      final off = !slots.contains(schedule[i]);
                      final live = isToday && !off;
                      return EnterFade(
                        index: i,
                        child: Opacity(
                          opacity: off ? 0.45 : 1,
                          child: _ClassRow(
                            slot: schedule[i],
                            status: _statusFor(
                              schedule[i],
                              current,
                              next,
                              minuteNow,
                              live,
                            ),
                            record: attendanceForSlot(attendance, schedule[i]),
                            minuteNow: live ? minuteNow : null,
                            isFirst: i == 0,
                            isLast: i == schedule.length - 1,
                          ),
                        ),
                      );
                    },
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }

  /// The next day after [from] that has classes, with its first class. With
  /// a calendar it looks further ahead, past exam weeks and breaks.
  ({DateTime date, TimetableSlot first})? _nextClassDay(DateTime from) {
    final horizon = calendar == null ? 7 : 28;
    for (var offset = 1; offset <= horizon; offset++) {
      final date = DateTime(from.year, from.month, from.day + offset);
      final slots = classesOnDate(slotsForDay(date.weekday), date, calendar);
      if (slots.isNotEmpty) return (date: date, first: slots.first);
    }
    return null;
  }
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

T? _firstWhereOrNull<T>(Iterable<T> items, bool Function(T) test) {
  for (final item in items) {
    if (test(item)) return item;
  }
  return null;
}

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
  final days = DateTime(
    date.year,
    date.month,
    date.day,
  ).difference(today).inDays;
  if (days == 1) return 'Tomorrow';
  if (days > 1 && days < 7) return DateFormat('EEEE').format(date);
  return DateFormat('EEE d MMM').format(date);
}

DateTime _at(DateTime day, String time) {
  final minutes = minutesOf(time);
  return DateTime(day.year, day.month, day.day, minutes ~/ 60, minutes % 60);
}

List<DateTime> _weekDates(DateTime now) {
  final monday = DateTime(
    now.year,
    now.month,
    now.day,
  ).subtract(Duration(days: now.weekday - DateTime.monday));
  return List.generate(7, (index) => monday.add(Duration(days: index)));
}

int _timeOfDay(DateTime value) => value.hour * 60 + value.minute;

/// [child] at reduced opacity when [dimmed], for classes called off.
Widget _dimmedIf(bool dimmed, Widget child) =>
    dimmed ? Opacity(opacity: 0.45, child: child) : child;
