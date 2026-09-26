import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

/// Fetches and parses in the bundled Rust library.
class LocalVtopBackend implements VtopBackend {
  LocalVtopBackend(this._client);

  final Future<VtopClient> Function() _client;

  @override
  String get name => 'local';

  /// Pages are fetched one by one on the device; nothing to do ahead.
  @override
  Future<void> prefetch(String semesterId, Set<VtopPage> pages) async {}

  @override
  Future<SemesterData> semesters() async =>
      vtop_api.fetchSemesters(client: await _client());

  @override
  Future<AttendanceData> attendance(String semesterId) async =>
      vtop_api.fetchAttendance(client: await _client(), semesterId: semesterId);

  @override
  Future<FullAttendanceData> fullAttendance({
    required String semesterId,
    required String courseId,
    required String courseType,
  }) async => vtop_api.fetchFullAttendance(
    client: await _client(),
    semesterId: semesterId,
    courseId: courseId,
    courseType: courseType,
  );

  @override
  Future<TimetableData> timetable(String semesterId) async =>
      vtop_api.fetchTimetable(client: await _client(), semesterId: semesterId);

  @override
  Future<MarksData> marks(String semesterId) async =>
      vtop_api.fetchMarks(client: await _client(), semesterId: semesterId);

  @override
  Future<ExamScheduleData> examSchedule(String semesterId) async => vtop_api
      .fetchExamSchedule(client: await _client(), semesterId: semesterId);

  @override
  Future<AcademicCalendarData> academicCalendar(String semesterId) async =>
      vtop_api.fetchAcademicCalendar(
        client: await _client(),
        semesterId: semesterId,
      );

  @override
  Future<GradeViewData> gradeView(String semesterId) async =>
      vtop_api.fetchGradeView(client: await _client(), semesterId: semesterId);

  @override
  Future<GradeDetailsData> gradeViewDetails({
    required String semesterId,
    required String courseId,
  }) async => vtop_api.fetchGradeViewDetails(
    client: await _client(),
    semesterId: semesterId,
    courseId: courseId,
  );

  @override
  Future<GradeHistoryData> gradeHistory() async =>
      vtop_api.fetchGradeHistory(client: await _client());

  @override
  Future<BiometricData> biometricHistory(String date) async =>
      vtop_api.fetchBiometricHistory(client: await _client(), date: date);
}
