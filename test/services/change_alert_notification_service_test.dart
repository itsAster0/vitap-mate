import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/services/change_alert_notification_service.dart';

void main() {
  group('stable hash', () {
    test('matches FNV-1a 64-bit reference vectors', () {
      expect(
        ChangeAlertNotificationService.stableHashForTest(''),
        0xcbf29ce484222325,
      );
      expect(
        ChangeAlertNotificationService.stableHashForTest('a'),
        0xaf63dc4c8601ec8c,
      );
    });

    test('is deterministic and content-sensitive', () {
      final a = ChangeAlertNotificationService.stableHashForTest(
        'Attendance below 75%|CSE101: 76.0% \u2192 74.9%',
      );
      expect(
        a,
        ChangeAlertNotificationService.stableHashForTest(
          'Attendance below 75%|CSE101: 76.0% \u2192 74.9%',
        ),
      );
      expect(a, isNot(equals(0)));
    });

    test('notification id fits positive int32 range', () {
      const inputs = ['change_alerts_v1|marks|VL20262', 'x', ''];
      for (final input in inputs) {
        final id = ChangeAlertNotificationService.stableHashForTest(input) &
            0x7fffffff;
        expect(id, greaterThanOrEqualTo(0));
        expect(id, lessThanOrEqualTo(0x7fffffff));
      }
    });
  });
}
