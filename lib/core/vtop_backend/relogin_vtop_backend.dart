import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Signs in again and retries once when a fetch fails with
/// [VtopError.sessionExpired].
///
/// The app's "logged in" flag can be stale: VTOP may drop a session the
/// device still believes in. Without this, a refresh would stop at
/// "Your saved session expired" instead of recovering.
///
/// When several fetches fail at once they share one re-login, and a fetch
/// that started before a re-login finished just retries on the new session.
class ReloginVtopBackend implements VtopBackend {
  ReloginVtopBackend(this._inner, {required Future<void> Function() relogin})
    : _relogin = relogin;

  final VtopBackend _inner;
  final Future<void> Function() _relogin;

  /// Bumped after every successful re-login.
  int _generation = 0;
  Future<void>? _inFlight;

  @override
  String get name => _inner.name;

  Future<T> _guard<T>(Future<T> Function() fetch) async {
    final startedAt = _generation;
    try {
      return await fetch();
    } on VtopError_SessionExpired {
      if (_generation == startedAt) {
        await (_inFlight ??= _signInAgain());
      }
      return fetch();
    }
  }

  Future<void> _signInAgain() async {
    try {
      await _relogin();
      _generation++;
    } finally {
      _inFlight = null;
    }
  }

  @override
  Future<void> prefetch(String semesterId, Set<VtopPage> pages) =>
      _guard(() => _inner.prefetch(semesterId, pages));

  @override
  Future<SemesterData> semesters() => _guard(_inner.semesters);

  @override
  Future<AttendanceData> attendance(String semesterId) =>
      _guard(() => _inner.attendance(semesterId));

  @override
  Future<FullAttendanceData> fullAttendance({
    required String semesterId,
    required String courseId,
    required String courseType,
  }) => _guard(
    () => _inner.fullAttendance(
      semesterId: semesterId,
      courseId: courseId,
      courseType: courseType,
    ),
  );

  @override
  Future<TimetableData> timetable(String semesterId) =>
      _guard(() => _inner.timetable(semesterId));

  @override
  Future<MarksData> marks(String semesterId) =>
      _guard(() => _inner.marks(semesterId));

  @override
  Future<ExamScheduleData> examSchedule(String semesterId) =>
      _guard(() => _inner.examSchedule(semesterId));

  @override
  Future<GradeViewData> gradeView(String semesterId) =>
      _guard(() => _inner.gradeView(semesterId));

  @override
  Future<GradeDetailsData> gradeViewDetails({
    required String semesterId,
    required String courseId,
  }) => _guard(
    () => _inner.gradeViewDetails(semesterId: semesterId, courseId: courseId),
  );

  @override
  Future<GradeHistoryData> gradeHistory() => _guard(_inner.gradeHistory);

  @override
  Future<BiometricData> biometricHistory(String date) =>
      _guard(() => _inner.biometricHistory(date));
}
