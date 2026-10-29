import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

void main() {
  String label(String key) => vtopTaskLabel('vtop_$key');

  test('every page fetch has its own label', () {
    expect(
      label(savedPageKey(VtopQueries.attendance('S'))),
      'Fetching attendance...',
    );
    expect(
      label(
        savedPageKey(
          VtopQueries.fullAttendance('S', courseId: 'C', courseType: 'ETH'),
        ),
      ),
      'Fetching attendance details...',
    );
    expect(label(savedPageKey(VtopQueries.grades('S'))), 'Fetching grades...');
    expect(
      label(savedPageKey(VtopQueries.gradeDetails('S', courseId: 'C'))),
      'Fetching grade details...',
    );
    expect(
      label(savedPageKey(VtopQueries.examSchedule('S'))),
      'Fetching exam schedule...',
    );
    expect(label(savedPageKey(VtopQueries.profile())), 'Fetching profile...');
    expect(vtopTaskLabel('vtop_login_X'), 'Logging in to VTOP...');
    expect(
      label(savedPageKey(VtopQueries.coursePageCourses('S'))),
      'Fetching courses...',
    );
    expect(
      label(savedPageKey(VtopQueries.coursePageClasses('S', courseId: 'C'))),
      'Fetching course sections...',
    );
    expect(
      label(savedPageKey(VtopQueries.generalOuting())),
      'Fetching outings...',
    );
    expect(
      label(savedPageKey(VtopQueries.vtopSession())),
      'Preparing download...',
    );
    expect(
      vtopTaskLabel('vtop_cancel_general_outing_1'),
      'Cancelling outing...',
    );
  });

  test('an unknown task still says what it is', () {
    expect(vtopTaskLabel('vtop_course_page_x'), 'Fetching course page x...');
  });
}
