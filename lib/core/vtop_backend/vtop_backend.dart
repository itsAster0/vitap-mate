import 'package:vitapmate/src/api/vtop/types.dart';

/// Where VTOP data comes from. Repositories fetch through this and do not
/// know whether the page is fetched and parsed on the device ([LocalVtopBackend])
/// or by a vtop-server ([RemoteVtopBackend]).
///
/// Login always happens on the device; a remote backend reuses the device's
/// VTOP session cookie.
abstract interface class VtopBackend {
  /// Short name for logs: `local` or `server`.
  String get name;

  /// Fetches [pages] for [semesterId] ahead of the individual calls, when the
  /// backend can do that more cheaply in one go (vtop-server's
  /// `/v1/refresh`). The following fetch calls then use the results. A
  /// no-op for the local backend.
  Future<void> prefetch(String semesterId, Set<VtopPage> pages);

  Future<SemesterData> semesters();
  Future<AttendanceData> attendance(String semesterId);
  Future<FullAttendanceData> fullAttendance({
    required String semesterId,
    required String courseId,
    required String courseType,
  });
  Future<TimetableData> timetable(String semesterId);
  Future<MarksData> marks(String semesterId);
  Future<ExamScheduleData> examSchedule(String semesterId);
  Future<GradeViewData> gradeView(String semesterId);
  Future<GradeDetailsData> gradeViewDetails({
    required String semesterId,
    required String courseId,
  });
  Future<GradeHistoryData> gradeHistory();

  /// [date] is `DD/MM/YYYY`.
  Future<BiometricData> biometricHistory(String date);
}

/// Pages [VtopBackend.prefetch] can fetch ahead.
enum VtopPage {
  semesters('semesters'),
  attendance('attendance'),
  timetable('timetable'),
  marks('marks'),
  examSchedule('exam_schedule'),
  grades('grades'),

  /// Per-class attendance of every course (one VTOP request per course on
  /// the server, but a single round trip for the phone).
  fullAttendance('full_attendance');

  const VtopPage(this.serverName);

  /// The part name vtop-server's `/v1/refresh` uses.
  final String serverName;
}
