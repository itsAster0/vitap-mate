import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/vtop_backend/vtop_queries.dart';
import 'package:vitapmate/core/vtop_backend/vtop_query.dart';

void main() {
  test('keys match the shared vectors', () {
    // The same file vtop-shared's key tests read.
    final vectors =
        jsonDecode(
              File(
                'rust/vtop-shared/tests/fixtures/page_keys.json',
              ).readAsStringSync(),
            )
            as List<dynamic>;
    expect(vectors, isNotEmpty);
    for (final vector in vectors.cast<Map<String, dynamic>>()) {
      final args = (vector['args'] as Map<String, dynamic>)
          .cast<String, String>();
      expect(pageKey(vector['name'] as String, args), vector['key']);
    }
  });

  test('catalog queries carry their key and semester', () {
    expect(VtopQueries.timetable('S').key, 'timetable');
    expect(VtopQueries.timetable('S').semester, 'S');
    expect(
      VtopQueries.fullAttendance('S', courseId: 'A', courseType: 'ETH').key,
      'full_attendance?course_id=A&course_type=ETH',
    );
    expect(VtopQueries.semesters().semester, isNull);
  });
}
