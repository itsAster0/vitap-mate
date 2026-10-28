/// Every VTOP page the app reads, described once ([VtopQuery]). A new page
/// is one entry here plus one in the engine's registry
/// (`rust/vtop-bridge/src/pages.rs`).
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

abstract final class VtopQueries {
  /// The semester list is not per semester; [semester] only groups it
  /// with that semester's refresh.
  static VtopQuery<SemesterData> semesters({String? semester}) => VtopQuery(
    name: 'semesters',
    semester: semester,
    fromJson: SemesterData.fromJson,
    local: (client) => vtop_api.fetchSemesters(client: client),
  );

  static VtopQuery<AttendanceData> attendance(String semester) => VtopQuery(
    name: 'attendance',
    semester: semester,
    fromJson: AttendanceData.fromJson,
    local: (client) =>
        vtop_api.fetchAttendance(client: client, semesterId: semester),
  );

  static VtopQuery<FullAttendanceData> fullAttendance(
    String semester, {
    required String courseId,
    required String courseType,
  }) => VtopQuery(
    name: 'full_attendance',
    semester: semester,
    args: {'course_id': courseId, 'course_type': courseType},
    fromJson: FullAttendanceData.fromJson,
    local: (client) => vtop_api.fetchFullAttendance(
      client: client,
      semesterId: semester,
      courseId: courseId,
      courseType: courseType,
    ),
  );

  static VtopQuery<TimetableData> timetable(String semester) => VtopQuery(
    name: 'timetable',
    semester: semester,
    fromJson: TimetableData.fromJson,
    local: (client) =>
        vtop_api.fetchTimetable(client: client, semesterId: semester),
  );

  static VtopQuery<MarksData> marks(String semester) => VtopQuery(
    name: 'marks',
    semester: semester,
    fromJson: MarksData.fromJson,
    local: (client) =>
        vtop_api.fetchMarks(client: client, semesterId: semester),
  );

  static VtopQuery<ExamScheduleData> examSchedule(String semester) => VtopQuery(
    name: 'exam_schedule',
    semester: semester,
    fromJson: ExamScheduleData.fromJson,
    local: (client) =>
        vtop_api.fetchExamSchedule(client: client, semesterId: semester),
  );

  static VtopQuery<GradeViewData> grades(String semester) => VtopQuery(
    name: 'grades',
    semester: semester,
    fromJson: GradeViewData.fromJson,
    local: (client) =>
        vtop_api.fetchGradeView(client: client, semesterId: semester),
  );

  static VtopQuery<AcademicCalendarData> academicCalendar(String semester) =>
      VtopQuery(
        name: 'academic_calendar',
        semester: semester,
        fromJson: AcademicCalendarData.fromJson,
        local: (client) => vtop_api.fetchAcademicCalendar(
          client: client,
          semesterId: semester,
        ),
      );

  static VtopQuery<GradeDetailsData> gradeDetails(
    String semester, {
    required String courseId,
  }) => VtopQuery(
    name: 'grade_details',
    semester: semester,
    args: {'course_id': courseId},
    fromJson: GradeDetailsData.fromJson,
    local: (client) => vtop_api.fetchGradeViewDetails(
      client: client,
      semesterId: semester,
      courseId: courseId,
    ),
  );

  static VtopQuery<GradeHistoryData> gradeHistory() => VtopQuery(
    name: 'grade_history',
    fromJson: GradeHistoryData.fromJson,
    local: (client) => vtop_api.fetchGradeHistory(client: client),
  );

  /// [date] is `DD/MM/YYYY`.
  static VtopQuery<BiometricData> biometric(String date) => VtopQuery(
    name: 'biometric',
    args: {'date': date},
    fromJson: BiometricData.fromJson,
    local: (client) =>
        vtop_api.fetchBiometricHistory(client: client, date: date),
  );

  static VtopQuery<StudentProfile> profile() => VtopQuery(
    name: 'profile',
    fromJson: StudentProfile.fromJson,
    local: (client) => vtop_api.fetchProfile(client: client),
  );

  /// The mentor's details and the messages they sent.
  static VtopQuery<MentorData> mentor() => VtopQuery(
    name: 'mentor',
    fromJson: MentorData.fromJson,
    local: (client) => vtop_api.fetchMentor(client: client),
  );

  // Pages not saved on the phone: fetched in full every time.

  static VtopQuery<GeneralOutingData> generalOuting() => VtopQuery(
    name: 'general_outing',
    fromJson: _generalOuting,
    local: (client) => vtop_api.fetchGeneralOuting(client: client),
  );

  static VtopQuery<WeekendOutingData> weekendOuting() => VtopQuery(
    name: 'weekend_outing',
    fromJson: _weekendOuting,
    local: (client) => vtop_api.fetchWeekendOuting(client: client),
  );

  /// An accepted outing's PDF pass.
  static VtopQuery<Uint8List> outingPass({
    required bool weekend,
    required String passId,
  }) => VtopQuery(
    name: 'outing_pass',
    args: {'kind': weekend ? 'weekend' : 'general', 'pass_id': passId},
    fromJson: (json) => base64.decode(json['pdf'] as String),
    local: (client) => vtop_api.fetchOutingPass(
      client: client,
      weekend: weekend,
      passId: passId,
    ),
  );

  static VtopQuery<CoursePageCourses> coursePageCourses(String semester) =>
      VtopQuery(
        name: 'course_page_courses',
        semester: semester,
        fromJson: CoursePageCourses.fromJson,
        local: (client) => vtop_api.fetchCoursePageCourses(
          client: client,
          semesterId: semester,
        ),
      );

  static VtopQuery<CoursePageClasses> coursePageClasses(
    String semester, {
    required String courseId,
  }) => VtopQuery(
    name: 'course_page_classes',
    semester: semester,
    args: {'course_id': courseId},
    fromJson: CoursePageClasses.fromJson,
    local: (client) => vtop_api.fetchCoursePageClasses(
      client: client,
      semesterId: semester,
      courseId: courseId,
    ),
  );

  static VtopQuery<CoursePageDetail> coursePageDetail(
    String semester, {
    required String erpId,
    required String classId,
  }) => VtopQuery(
    name: 'course_page_detail',
    semester: semester,
    args: {'erp_id': erpId, 'class_id': classId},
    fromJson: CoursePageDetail.fromJson,
    local: (client) => vtop_api.fetchCoursePageDetail(
      client: client,
      semesterId: semester,
      erpId: erpId,
      classId: classId,
    ),
  );

  static VtopQuery<CourseFileInfo> courseFileInfo(String path) => VtopQuery(
    name: 'course_file_info',
    args: {'path': path},
    fromJson: CourseFileInfo.fromJson,
    local: (client) => vtop_api.fetchCourseFileInfo(client: client, path: path),
  );

  /// A live VTOP session (cookies, CSRF token, registration number) for
  /// what must go from the phone to VTOP itself, like course file
  /// downloads. Through the server it is the server's session, so the
  /// phone does not sign in; on the phone, an unsigned-in client is
  /// `sessionExpired`, which signs in.
  static VtopQuery<SessionState> vtopSession() => VtopQuery(
    name: 'vtop_session',
    fromJson: (json) => SessionState(
      cookies: '${json['cookies'] ?? ''}',
      csrfToken: json['csrfToken'] as String?,
      registrationNumber: json['registrationNumber'] as String?,
    ),
    local: (client) async {
      if (!await vtop_api.fetchIsAuth(client: client)) {
        throw const VtopError.sessionExpired();
      }
      return vtop_api.exportSessionState(client: client);
    },
  );

  static VtopQuery<CourseFileInfo> coursePlanInfo(
    String semester, {
    required String classId,
  }) => VtopQuery(
    name: 'course_plan_info',
    semester: semester,
    args: {'class_id': classId},
    fromJson: CourseFileInfo.fromJson,
    local: (client) => vtop_api.fetchCoursePlanInfo(
      client: client,
      semesterId: semester,
      classId: classId,
    ),
  );
}

/// What changes VTOP. Each is sent once; see [VtopBackend.run].
abstract final class VtopActions {
  /// [outDate], [inDate] are VTOP's `DD-Mon-YYYY`.
  static VtopAction<OutingApplyResult> applyGeneralOuting({
    required String place,
    required String purpose,
    required String outDate,
    required int outHour,
    required int outMinute,
    required String inDate,
    required int inHour,
    required int inMinute,
  }) => VtopAction(
    name: 'apply_general_outing',
    args: {
      'place': place,
      'purpose': purpose,
      'out_date': outDate,
      'out_time': _hhmm(outHour, outMinute),
      'in_date': inDate,
      'in_time': _hhmm(inHour, inMinute),
    },
    fromJson: OutingApplyResult.fromJson,
    local: (client) => vtop_api.applyGeneralOuting(
      client: client,
      place: place,
      purpose: purpose,
      outDate: outDate,
      outHour: outHour,
      outMinute: outMinute,
      inDate: inDate,
      inHour: inHour,
      inMinute: inMinute,
    ),
  );

  /// [place] and [timeSlot] are option values from [WeekendOutingData].
  static VtopAction<OutingApplyResult> applyWeekendOuting({
    required String place,
    required String purpose,
    required String date,
    required String timeSlot,
    required String contactNumber,
  }) => VtopAction(
    name: 'apply_weekend_outing',
    args: {
      'place': place,
      'purpose': purpose,
      'date': date,
      'time_slot': timeSlot,
      'contact_number': contactNumber,
    },
    fromJson: OutingApplyResult.fromJson,
    local: (client) => vtop_api.applyWeekendOuting(
      client: client,
      place: place,
      purpose: purpose,
      date: date,
      timeSlot: timeSlot,
      contactNumber: contactNumber,
    ),
  );

  static VtopAction<OutingCancelResult> cancelGeneralOuting(String leaveId) =>
      VtopAction(
        name: 'cancel_general_outing',
        args: {'leave_id': leaveId},
        fromJson: OutingCancelResult.fromJson,
        local: (client) =>
            vtop_api.cancelGeneralOuting(client: client, leaveId: leaveId),
      );

  static VtopAction<OutingCancelResult> cancelWeekendOuting(String bookingId) =>
      VtopAction(
        name: 'cancel_weekend_outing',
        args: {'booking_id': bookingId},
        fromJson: OutingCancelResult.fromJson,
        local: (client) =>
            vtop_api.cancelWeekendOuting(client: client, bookingId: bookingId),
      );
}

String _hhmm(int hour, int minute) =>
    '${'$hour'.padLeft(2, '0')}:${'$minute'.padLeft(2, '0')}';

// The outing forms hold byte lists (offered hours, weekdays), which the
// generated JSON code cannot read; the rest of them is generated.

Uint8List _bytes(Object? value) =>
    Uint8List.fromList([for (final b in value as List) (b as num).toInt()]);

BigInt _time(Object? value) => BigInt.parse('$value');

OutingStudent? _student(Object? value) => value == null
    ? null
    : OutingStudent.fromJson(value as Map<String, dynamic>);

List<T> _list<T>(Object? value, T Function(Map<String, dynamic>) item) => [
  for (final e in value as List) item(e as Map<String, dynamic>),
];

GeneralOutingData _generalOuting(Map<String, dynamic> json) =>
    GeneralOutingData(
      student: _student(json['student']),
      notice: json['notice'] as String,
      records: _list(json['records'], GeneralOutingRecord.fromJson),
      outHours: _bytes(json['outHours']),
      inHours: _bytes(json['inHours']),
      placeMaxLength: json['placeMaxLength'] as int,
      purposeMaxLength: json['purposeMaxLength'] as int,
      maxDaysAhead: json['maxDaysAhead'] as int,
      maxDaysAway: json['maxDaysAway'] as int,
      updateTime: _time(json['updateTime']),
    );

WeekendOutingData _weekendOuting(Map<String, dynamic> json) =>
    WeekendOutingData(
      student: _student(json['student']),
      notice: json['notice'] as String,
      records: _list(json['records'], WeekendOutingRecord.fromJson),
      places: _list(json['places'], OutingOption.fromJson),
      timeSlots: _list(json['timeSlots'], OutingOption.fromJson),
      purposeMaxLength: json['purposeMaxLength'] as int,
      maxDaysAhead: json['maxDaysAhead'] as int,
      weekdays: _bytes(json['weekdays']),
      updateTime: _time(json['updateTime']),
    );
