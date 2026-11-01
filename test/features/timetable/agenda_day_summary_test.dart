import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/features/timetable/logic/time_format.dart';
import 'package:vitapmate/features/timetable/widgets/agenda_timetable_view.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

TimetableSlot _slot(String code, String start, String end) => TimetableSlot(
  serial: '1',
  day: slotDays[DateTime.now().weekday - 1],
  slot: 'A1',
  courseCode: code,
  courseType: 'ETH',
  roomNo: '101',
  block: 'CB',
  startTime: start,
  endTime: end,
  name: '$code course',
  kind: ClassKind.theory,
  faculty: '',
  credits: '3',
);

void main() {
  testWidgets('after the last class the card recaps the day', (tester) async {
    // Two classes just after midnight, so both are over when this runs.
    final slots = [
      _slot('AAA1001', '00:00', '00:01'),
      _slot('BBB1001', '00:02', '00:03'),
    ];
    final today = DateFormat('dd-MM-yyyy').format(DateTime.now());
    await tester.pumpWidget(
      MaterialApp(
        builder: (_, child) =>
            FTheme(data: FTheme.neutral.dark.touch, child: child!),
        home: SingleChildScrollView(
          child: AgendaTimetableView(
            selectedDay: ValueNotifier(DateTime.now().weekday),
            classDays: {DateTime.now().weekday},
            slotsForDay: (day) =>
                day == DateTime.now().weekday ? slots : const [],
            history: {
              // AAA1001 is posted present; BBB1001 isn't posted yet.
              ('AAA1001', false): FullAttendanceData(
                records: [
                  FullAttendanceRecord(
                    serial: '1',
                    date: today,
                    slot: 'A1',
                    dayTime: '',
                    status: 'Present',
                    remark: '',
                  ),
                ],
                semesterId: 'S',
                updateTime: BigInt.zero,
                courseId: 'C',
                courseType: 'ETH',
              ),
            },
          ),
        ),
      ),
    );
    await tester.pump(const Duration(seconds: 2));

    expect(find.text('Done for today'), findsOneWidget);
    expect(
      find.textContaining('1 present', findRichText: true),
      findsOneWidget,
    );
    expect(
      find.textContaining('1 not posted', findRichText: true),
      findsOneWidget,
    );
    // No "NEXT" row on the finished card.
    expect(find.text('NEXT'), findsNothing);
  });
}
