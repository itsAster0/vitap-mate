import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;
import 'package:vitapmate/core/vtop_backend/server_json.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_server_client.dart';
import 'package:vitapmate/core/vtop_backend/vtop_server_settings.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Returns the device's current VTOP session (cookie header, CSRF token and
/// registration number), or null when there is no signed-in session.
typedef VtopSessionSource = Future<SessionState?> Function();

/// A page fetched ahead by [RemoteVtopBackend.prefetch], waiting to be
/// handed to the matching fetch call.
class _Prefetched {
  _Prefetched(this.semesterId, this.at, this.entry);

  final String semesterId;
  final DateTime at;

  /// `{"data": …}` or `{"error": {"code", "message"}}`, in Rust shape.
  final Map<String, dynamic> entry;
}

/// Fetches through a vtop-server. The server parses the page with the same
/// Rust code and returns the same types; errors come back as [VtopError] so
/// callers handle both backends alike (for example, re-login on
/// [VtopError.sessionExpired]).
class RemoteVtopBackend implements VtopBackend {
  RemoteVtopBackend({
    required VtopServerSettings settings,
    required VtopSessionSource session,
    Future<void> Function(SessionState)? onSessionChanged,
    http.Client? httpClient,
    Duration timeout = const Duration(seconds: 60),
  }) : _server = VtopServerClient(
         settings,
         httpClient: httpClient,
         timeout: timeout,
       ),
       _deviceSession = session,
       _onSessionChanged = onSessionChanged;

  final VtopServerClient _server;
  final VtopSessionSource _deviceSession;

  /// Called when the server's reply carries different cookies (or CSRF
  /// token) than were sent, so the phone keeps the session VTOP now
  /// expects instead of reusing stale cookies.
  final Future<void> Function(SessionState)? _onSessionChanged;

  /// How long a prefetched page may wait for its fetch call.
  static const prefetchLifetime = Duration(minutes: 2);

  /// The session the server last sent back. Used only when the device does
  /// not know its CSRF token or registration number yet, and only while the
  /// cookie is unchanged.
  Map<String, dynamic>? _serverSession;
  final Map<VtopPage, _Prefetched> _prefetched = {};

  /// Prefetched per-course attendance, keyed by
  /// `semester|course_id|course_type`, each handed out once.
  final Map<String, _Prefetched> _prefetchedFull = {};

  static String _fullKey(String semester, String course, String type) =>
      '$semester|$course|$type';

  @override
  String get name => 'server';

  /// The session in vtop-server's shape. With the CSRF token and
  /// registration number included the server posts straight to the page
  /// instead of first validating the cookie with VTOP.
  Future<Map<String, Object?>> _session() async {
    final device = await _deviceSession();
    final cookies = device?.cookies.trim() ?? '';
    if (device == null || cookies.isEmpty) {
      throw const VtopError.sessionExpired();
    }
    if (device.csrfToken != null && device.registrationNumber != null) {
      return sessionToServerJson(device);
    }
    final known = _serverSession;
    if (known != null && known['cookies'] == cookies) return known;
    return {'cookies': cookies};
  }

  /// Cookie pairs as a set, since the server may list them in another order.
  static Set<String> _cookiePairs(Object? header) => '${header ?? ''}'
      .split(';')
      .map((pair) => pair.trim())
      .where((pair) => pair.isNotEmpty)
      .toSet();

  Future<void> _remember(
    http.Response response,
    Map<String, Object?> sent,
  ) async {
    final header = response.headers['x-vtop-session'];
    if (header == null || header.isEmpty) return;
    final Map<String, dynamic> returned;
    try {
      final padded = header.padRight((header.length + 3) ~/ 4 * 4, '=');
      final decoded = jsonDecode(utf8.decode(base64Url.decode(padded)));
      if (decoded is! Map<String, dynamic>) return;
      returned = decoded;
    } on FormatException {
      // A malformed header only costs a validation round trip next time.
      return;
    }
    _serverSession = returned;

    final returnedCookies = _cookiePairs(returned['cookies']);
    final sentCookies = _cookiePairs(sent['cookies']);
    final cookiesChanged =
        returnedCookies.isNotEmpty &&
        (returnedCookies.length != sentCookies.length ||
            !returnedCookies.containsAll(sentCookies));
    final csrfChanged =
        sent['csrf_token'] != null &&
        returned['csrf_token'] != null &&
        returned['csrf_token'] != sent['csrf_token'];
    if ((cookiesChanged || csrfChanged) && _onSessionChanged != null) {
      try {
        await _onSessionChanged(sessionFromServerJson(returned));
      } catch (_) {
        // Keeping the old session only means one more validation later.
      }
    }
  }

  Future<Map<String, dynamic>> _post(
    String path,
    Map<String, Object?> params, {
    Duration? timeout,
  }) async {
    try {
      final session = await _session();
      final (body, response) = await _server.post(path, {
        'session': session,
        ...params,
      }, timeout: timeout);
      await _remember(response, session);
      return body;
    } on VtopError_SessionExpired {
      _serverSession = null;
      _prefetched.clear();
      _prefetchedFull.clear();
      rethrow;
    }
  }

  Future<Map<String, dynamic>> _fetch(
    VtopPage page,
    String path,
    Map<String, Object?> params,
  ) async {
    final ready = _takePrefetched(page, params['semester_id'] as String?);
    final raw = ready ?? await _post(path, params);
    return rustJsonToDart(raw) as Map<String, dynamic>;
  }

  /// Hands out a prefetched page once, if it is recent and for the same
  /// semester. A prefetched error is thrown as the fetch's error.
  Map<String, dynamic>? _takePrefetched(VtopPage page, String? semesterId) {
    final found = _prefetched.remove(page);
    if (found == null) return null;
    final fresh = DateTime.now().difference(found.at) < prefetchLifetime;
    final sameSemester =
        page == VtopPage.semesters || found.semesterId == semesterId;
    if (!fresh || !sameSemester) return null;
    final data = found.entry['data'];
    if (data is Map<String, dynamic>) return data;
    final error = found.entry['error'];
    if (error is Map) {
      throw VtopServerClient.errorForCode(
        '${error['code'] ?? ''}',
        '${error['message'] ?? ''}',
      );
    }
    return null;
  }

  @override
  Future<void> prefetch(String semesterId, Set<VtopPage> pages) async {
    if (pages.isEmpty) return;
    final Map<String, dynamic> body;
    try {
      body = await _post('refresh', {
        'semester_id': semesterId,
        'include': [for (final page in pages) page.serverName],
      }, timeout: const Duration(seconds: 150));
    } on VtopError_SessionExpired {
      rethrow;
    } on VtopError {
      // The single fetches will try again one by one.
      return;
    }
    final now = DateTime.now();
    for (final page in pages) {
      final entry = body[page.serverName];
      if (entry is! Map<String, dynamic>) continue;
      if (page == VtopPage.fullAttendance) {
        // A list of details; courses missing from it are fetched alone.
        final details = entry['data'];
        if (details is! List) continue;
        for (final detail in details.whereType<Map<String, dynamic>>()) {
          final key = _fullKey(
            semesterId,
            '${detail['course_id']}',
            '${detail['course_type']}',
          );
          _prefetchedFull[key] = _Prefetched(semesterId, now, {'data': detail});
        }
      } else {
        _prefetched[page] = _Prefetched(semesterId, now, entry);
      }
    }
  }

  @override
  Future<SemesterData> semesters() async =>
      SemesterData.fromJson(await _fetch(VtopPage.semesters, 'semesters', {}));

  @override
  Future<AttendanceData> attendance(String semesterId) async =>
      AttendanceData.fromJson(
        await _fetch(VtopPage.attendance, 'attendance', {
          'semester_id': semesterId,
        }),
      );

  @override
  Future<FullAttendanceData> fullAttendance({
    required String semesterId,
    required String courseId,
    required String courseType,
  }) async {
    final ready = _prefetchedFull.remove(
      _fullKey(semesterId, courseId, courseType),
    );
    final fresh =
        ready != null && DateTime.now().difference(ready.at) < prefetchLifetime;
    final raw = fresh
        ? ready.entry['data'] as Map<String, dynamic>
        : await _post('attendance/full', {
            'semester_id': semesterId,
            'course_id': courseId,
            'course_type': courseType,
          });
    return FullAttendanceData.fromJson(
      rustJsonToDart(raw) as Map<String, dynamic>,
    );
  }

  @override
  Future<TimetableData> timetable(String semesterId) async =>
      TimetableData.fromJson(
        await _fetch(VtopPage.timetable, 'timetable', {
          'semester_id': semesterId,
        }),
      );

  @override
  Future<MarksData> marks(String semesterId) async => MarksData.fromJson(
    await _fetch(VtopPage.marks, 'marks', {'semester_id': semesterId}),
  );

  @override
  Future<ExamScheduleData> examSchedule(String semesterId) async =>
      ExamScheduleData.fromJson(
        await _fetch(VtopPage.examSchedule, 'exam-schedule', {
          'semester_id': semesterId,
        }),
      );

  @override
  Future<AcademicCalendarData> academicCalendar(String semesterId) async =>
      AcademicCalendarData.fromJson(
        rustJsonToDart(
              await _post('academic-calendar', {'semester_id': semesterId}),
            )
            as Map<String, dynamic>,
      );

  @override
  Future<GradeViewData> gradeView(String semesterId) async =>
      GradeViewData.fromJson(
        await _fetch(VtopPage.grades, 'grades', {'semester_id': semesterId}),
      );

  @override
  Future<GradeDetailsData> gradeViewDetails({
    required String semesterId,
    required String courseId,
  }) async => GradeDetailsData.fromJson(
    rustJsonToDart(
          await _post('grades/details', {
            'semester_id': semesterId,
            'course_id': courseId,
          }),
        )
        as Map<String, dynamic>,
  );

  @override
  Future<GradeHistoryData> gradeHistory() async => GradeHistoryData.fromJson(
    rustJsonToDart(await _post('grade-history', const {}))
        as Map<String, dynamic>,
  );

  @override
  Future<BiometricData> biometricHistory(String date) async =>
      BiometricData.fromJson(
        rustJsonToDart(await _post('biometric', {'date': date}))
            as Map<String, dynamic>,
      );
}

/// Result of [checkVtopServer]: a server version when everything worked,
/// otherwise a message for the user.
class VtopServerCheck {
  const VtopServerCheck.ok(this.version) : error = null;
  const VtopServerCheck.failed(this.error) : version = null;

  final String? version;
  final String? error;

  bool get isOk => error == null;
}

/// Checks that [settings] points at a vtop-server and that it accepts the
/// API key (or no key, for an open server). Does not touch VTOP or send any
/// cookie.
Future<VtopServerCheck> checkVtopServer(
  VtopServerSettings settings, {
  http.Client? httpClient,
  Duration timeout = const Duration(seconds: 10),
}) async {
  final client = httpClient ?? http.Client();
  try {
    final base = settings.endpoint('health').resolve('../health');
    final health = await client.get(base).timeout(timeout);
    final body = health.statusCode == 200 ? jsonDecode(health.body) : null;
    if (body is! Map || body['status'] != 'ok') {
      return const VtopServerCheck.failed(
        'No vtop-server answered at that URL.',
      );
    }
    // An empty session is a cheap keyed request: 400 means the key was
    // accepted, 401 means it was not.
    final probe = await client
        .post(
          settings.endpoint('semesters'),
          headers: settings.headers,
          body: jsonEncode({
            'session': {'cookies': ''},
          }),
        )
        .timeout(timeout);
    if (probe.statusCode == 401) {
      return VtopServerCheck.failed(
        settings.hasApiKey
            ? 'The server rejected this API key.'
            : 'This server needs an API key.',
      );
    }
    if (probe.statusCode == 429) {
      return const VtopServerCheck.failed(
        'The server is rate limiting this key. Try again in a minute.',
      );
    }
    return VtopServerCheck.ok('${body['version'] ?? '?'}');
  } on TimeoutException {
    return const VtopServerCheck.failed('The server did not answer in time.');
  } on FormatException {
    return const VtopServerCheck.failed('No vtop-server answered at that URL.');
  } on SocketException {
    return const VtopServerCheck.failed('Could not reach the server.');
  } on http.ClientException {
    return const VtopServerCheck.failed('Could not reach the server.');
  } finally {
    if (httpClient == null) client.close();
  }
}
