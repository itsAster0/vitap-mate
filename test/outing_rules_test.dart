import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/outing/domain/outing_rules.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// What VTOP's general outing page offered on 2026-10-07.
GeneralOutingData generalForm() => GeneralOutingData(
  notice: '',
  records: const [],
  outHours: Uint8List.fromList([for (var h = 6; h <= 22; h++) h]),
  inHours: Uint8List.fromList([for (var h = 6; h <= 20; h++) h]),
  placeMaxLength: 20,
  purposeMaxLength: 20,
  maxDaysAhead: 30,
  maxDaysAway: 15,
  updateTime: BigInt.zero,
);

WeekendOutingData weekendForm() => WeekendOutingData(
  notice: '',
  records: const [],
  places: const [OutingOption(value: 'Vijayawada', label: 'Vijayawada')],
  timeSlots: const [
    OutingOption(value: '9:30 AM- 3:30PM', label: '9:30 AM- 3:30PM'),
  ],
  purposeMaxLength: 20,
  maxDaysAhead: 6,
  weekdays: Uint8List.fromList([0, 1]),
  updateTime: BigInt.zero,
);

/// Wednesday 7 Oct 2026, 5:40 PM.
final now = DateTime(2026, 10, 7, 17, 40);

GeneralOutingCheck general({
  String place = 'Guntur',
  String purpose = 'Dentist',
  DateTime? leaving,
  DateTime? returning,
}) => checkGeneralOuting(
  form: generalForm(),
  now: now,
  place: place,
  purpose: purpose,
  leaving: leaving,
  returning: returning,
);

void main() {
  group('general outing', () {
    test('accepts a trip inside every limit', () {
      final check = general(
        leaving: DateTime(2026, 10, 9, 9),
        returning: DateTime(2026, 10, 9, 20),
      );
      expect(check.isValid, isTrue);
    });

    test('needs 24 hours notice', () {
      expect(
        general(
          leaving: DateTime(2026, 10, 8, 17, 39),
          returning: DateTime(2026, 10, 8, 20),
        ).leaving,
        contains('24 h notice'),
      );
      expect(
        general(
          leaving: DateTime(2026, 10, 8, 17, 40),
          returning: DateTime(2026, 10, 8, 20),
        ).leaving,
        isNull,
      );
    });

    test('leaves between 6 AM and 10 PM, back by 8 PM', () {
      final late = general(
        leaving: DateTime(2026, 10, 10, 22, 1),
        returning: DateTime(2026, 10, 11, 9),
      );
      expect(late.leaving, contains('10:00 PM'));
      expect(
        general(
          leaving: DateTime(2026, 10, 10, 22),
          returning: DateTime(2026, 10, 11, 9),
        ).leaving,
        isNull,
      );
      final backLate = general(
        leaving: DateTime(2026, 10, 10, 9),
        returning: DateTime(2026, 10, 10, 20, 30),
      );
      expect(backLate.returning, contains('8:00 PM'));
      expect(
        general(
          leaving: DateTime(2026, 10, 10, 5, 59),
          returning: DateTime(2026, 10, 10, 12),
        ).leaving,
        isNotNull,
      );
    });

    test('return has to be after leaving', () {
      expect(
        general(
          leaving: DateTime(2026, 10, 10, 15),
          returning: DateTime(2026, 10, 10, 14),
        ).returning,
        contains('after'),
      );
    });

    test('leaving within 30 days; 15-day trip is shown, not enforced', () {
      expect(
        general(
          leaving: DateTime(2026, 10, 10, 9),
          returning: DateTime(2026, 10, 26, 9),
        ).returning,
        isNull,
      );
      expect(
        general(
          leaving: DateTime(2026, 11, 7, 9),
          returning: DateTime(2026, 11, 7, 18),
        ).leaving,
        contains('30 days'),
      );
    });

    test('text has to be there and fit VTOP\'s 20 characters', () {
      expect(general(place: '  ').place, isNotNull);
      expect(general(purpose: 'a' * 21).purpose, contains('20'));
      expect(general(purpose: '  a   b  ').purpose, isNull);
    });
  });

  group('weekend outing', () {
    test('offers the allowed weekdays within the window', () {
      expect(weekendOutingDates(weekendForm(), now), [
        DateTime(2026, 10, 11),
        DateTime(2026, 10, 12),
      ]);
      expect(weekdayNames([0, 1]), 'Sundays and Mondays');
      // Never the same day: on Sunday only Monday is left.
      expect(weekendOutingDates(weekendForm(), DateTime(2026, 10, 11, 9)), [
        DateTime(2026, 10, 12),
      ]);
      // Saturday night still offers Sunday.
      expect(
        weekendOutingDates(weekendForm(), DateTime(2026, 10, 10, 23, 59)).first,
        DateTime(2026, 10, 11),
      );
    });

    test('checks day, purpose and contact', () {
      final ok = checkWeekendOuting(
        form: weekendForm(),
        now: now,
        purpose: 'Movie',
        date: DateTime(2026, 10, 11),
        contact: '9876543210',
      );
      expect(ok.isValid, isTrue);
      final bad = checkWeekendOuting(
        form: weekendForm(),
        now: now,
        purpose: '',
        date: DateTime(2026, 10, 13),
        contact: '98765',
      );
      expect(bad.purpose, isNotNull);
      expect(bad.date, isNotNull);
      expect(bad.contact, isNotNull);
    });
  });

  test('formats dates the way the form posts them', () {
    expect(vtopOutingDate(DateTime(2026, 10, 11)), '11-Oct-2026');
  });

  test('reads statuses, moments and slots', () {
    expect(outingStatus("Waiting for Warden's Approval"), OutingStatus.pending);
    expect(outingStatus('Leave Request Accepted'), OutingStatus.approved);
    expect(outingStatus('Outing Request Accepted'), OutingStatus.approved);
    expect(outingStatus('Rejected by Warden'), OutingStatus.rejected);
    expect(
      parseOutingMoment('2026-03-18', '01:00 PM'),
      DateTime(2026, 3, 18, 13),
    );
    expect(parseOutingMoment('2026-03-18', ''), DateTime(2026, 3, 18));
    expect(
      parseOutingMoment('2026-03-18', '12:15 AM'),
      DateTime(2026, 3, 18, 0, 15),
    );
    expect(slotLabel('12:30 PM- 6:30PM'), '12:30 PM – 6:30 PM');
  });

  test('says how long an outing lasts', () {
    expect(
      awayLabel(
        DateTime(2026, 10, 17, 11, 30),
        DateTime(2026, 10, 21, 20),
        hasReturnTime: true,
      ),
      '4 days',
    );
    expect(
      awayLabel(
        DateTime(2026, 10, 9, 23),
        DateTime(2026, 10, 10, 7),
        hasReturnTime: true,
      ),
      '1 day',
    );
    expect(
      awayLabel(
        DateTime(2026, 10, 10, 11, 30),
        DateTime(2026, 10, 10, 20),
        hasReturnTime: true,
      ),
      '8 h 30 min',
    );
    expect(
      awayLabel(
        DateTime(2026, 10, 10, 11),
        DateTime(2026, 10, 10),
        hasReturnTime: false,
      ),
      isNull,
    );
  });
}
