import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/core/vtop_backend/remote_vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/server_json.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_server_settings.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// JSON exactly as vtop-server sends it: the Rust parser snapshots.
Map<String, dynamic> rustSnapshot(String name) {
  final file = File('rust/vtop-core/tests/fixtures/$name.expected.json');
  return jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
}

Map<String, dynamic> dartJson(String name) =>
    rustJsonToDart(rustSnapshot(name)) as Map<String, dynamic>;

const settings = VtopServerSettings(
  url: 'https://vtop.example.com/',
  apiKey: 'test-key-0123456789',
);

void main() {
  group('server JSON matches the generated Dart types', () {
    test('every snapshot parses', () {
      final attendance = AttendanceData.fromJson(dartJson('attendance'));
      expect(attendance.records.first.courseId, 'AP2026271000123');
      expect(attendance.updateTime, BigInt.zero);

      final timetable = TimetableData.fromJson(dartJson('timetable'));
      expect(timetable.slots.first.kind, ClassKind.theory);
      expect(timetable.slots.any((s) => s.kind == ClassKind.lab), isTrue);
      expect(timetable.courses, hasLength(3));

      expect(
        FullAttendanceData.fromJson(dartJson('full_attendance')).records,
        hasLength(3),
      );
      expect(
        SemesterData.fromJson(dartJson('semesters')).semesters,
        hasLength(3),
      );
      expect(
        MarksData.fromJson(dartJson('marks')).records.first.marks,
        hasLength(2),
      );
      expect(
        ExamScheduleData.fromJson(dartJson('exam_schedule')).exams,
        hasLength(2),
      );
      expect(
        GradeViewData.fromJson(dartJson('grade_view')).courses,
        hasLength(2),
      );
      final details = GradeDetailsData.fromJson(dartJson('grade_details'));
      expect(details.gradeRanges, hasLength(7));
      expect(details.marks.first.weightageMark, '12.30');
      final history = GradeHistoryData.fromJson(dartJson('grade_history'));
      expect(history.student.regNo, '22BCE0000');
      expect(history.cgpa.sGrades, '1');
      expect(
        BiometricData.fromJson(dartJson('biometric')).records,
        hasLength(2),
      );
    });

    test('round-trips through toJson unchanged', () {
      final data = TimetableData.fromJson(dartJson('timetable'));
      final again = TimetableData.fromJson(
        jsonDecode(jsonEncode(data.toJson())) as Map<String, dynamic>,
      );
      expect(again, data);
    });
  });

  group('RemoteVtopBackend', () {
    test('posts the cookie with the API key and parses the reply', () async {
      late http.Request sent;
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient((request) async {
          sent = request;
          return http.Response(
            jsonEncode(rustSnapshot('attendance')),
            200,
            headers: {'content-type': 'application/json'},
          );
        }),
      );

      final data = await backend.attendance('AP2026271');

      expect(sent.url.toString(), 'https://vtop.example.com/v1/attendance');
      expect(sent.headers['x-api-key'], settings.apiKey);
      expect(jsonDecode(sent.body), {
        'session': {'cookies': 'JSESSIONID=abc'},
        'semester_id': 'AP2026271',
      });
      expect(data.records, hasLength(3));
    });

    test('sends the device CSRF token and regno from the first call', () async {
      late Map<String, dynamic> body;
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(
          cookies: 'JSESSIONID=abc',
          csrfToken: 'csrf-device',
          registrationNumber: '22BCE0000',
        ),
        httpClient: MockClient((request) async {
          body = jsonDecode(request.body) as Map<String, dynamic>;
          return http.Response(jsonEncode(rustSnapshot('marks')), 200);
        }),
      );

      await backend.marks('AP2026271');

      expect(body['session'], {
        'cookies': 'JSESSIONID=abc',
        'csrf_token': 'csrf-device',
        'registration_number': '22BCE0000',
      });
    });

    test('omits the API key header for a keyless server', () async {
      late http.Request sent;
      final backend = RemoteVtopBackend(
        settings: const VtopServerSettings(
          url: 'http://10.0.2.2:8080',
          apiKey: '',
        ),
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient((request) async {
          sent = request;
          return http.Response(jsonEncode(rustSnapshot('semesters')), 200);
        }),
      );

      await backend.semesters();

      expect(sent.headers.containsKey('x-api-key'), isFalse);
    });

    test('sends back the session the server returned', () async {
      final bodies = <Map<String, dynamic>>[];
      final serverSession = base64Url
          .encode(
            utf8.encode(
              jsonEncode({
                'cookies': 'JSESSIONID=abc',
                'csrf_token': 'csrf',
                'registration_number': '22BCE0000',
              }),
            ),
          )
          .replaceAll('=', '');
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient((request) async {
          bodies.add(jsonDecode(request.body) as Map<String, dynamic>);
          return http.Response(
            jsonEncode(rustSnapshot('semesters')),
            200,
            headers: {'x-vtop-session': serverSession},
          );
        }),
      );

      await backend.semesters();
      await backend.semesters();

      expect(bodies[0]['session'], {'cookies': 'JSESSIONID=abc'});
      expect(bodies[1]['session']['csrf_token'], 'csrf');
    });

    test('maps server errors to VtopError', () async {
      Future<Object?> errorFor(String code, int status) async {
        final backend = RemoteVtopBackend(
          settings: settings,
          session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
          httpClient: MockClient(
            (_) async => http.Response(
              jsonEncode({
                'error': {'code': code, 'message': 'm'},
              }),
              status,
            ),
          ),
        );
        try {
          await backend.marks('S');
        } catch (error) {
          return error;
        }
        return null;
      }

      expect(
        await errorFor('session_expired', 401),
        const VtopError.sessionExpired(),
      );
      expect(
        await errorFor('vtop_unreachable', 502),
        const VtopError.networkError(),
      );
      expect(
        await errorFor('unauthorized', 401),
        isA<VtopError_ConfigurationError>(),
      );
      expect(
        await errorFor('vtop_error', 502),
        isA<VtopError_VtopServerError>(),
      );
    });

    test('without a signed-in session it does not call the server', () async {
      var calls = 0;
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => null,
        httpClient: MockClient((_) async {
          calls++;
          return http.Response('{}', 200);
        }),
      );

      expect(backend.gradeHistory(), throwsA(const VtopError.sessionExpired()));
      await Future<void>.delayed(Duration.zero);
      expect(calls, 0);
    });

    test('network failures become VtopError.networkError', () async {
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient((_) async => throw http.ClientException('down')),
      );
      expect(backend.timetable('S'), throwsA(const VtopError.networkError()));
    });
  });

  group('VtopServerSettings', () {
    test('needs a URL; the key is optional', () {
      expect(VtopServerSettings.disabled.isEnabled, isFalse);
      const keyless = VtopServerSettings(url: 'https://x', apiKey: '');
      expect(keyless.isEnabled, isTrue);
      expect(keyless.hasApiKey, isFalse);
      expect(keyless.headers.containsKey('x-api-key'), isFalse);
      expect(settings.headers['x-api-key'], settings.apiKey);
    });

    test('switching off keeps the server details', () {
      final off = settings.copyWith(enabled: false);
      expect(off.isEnabled, isFalse);
      expect(off.hasUrl, isTrue);
      expect(off.apiKey, settings.apiKey);
      expect(off.copyWith(enabled: true).isEnabled, isTrue);
      expect(
        const VtopServerSettings(url: '', apiKey: '', enabled: true).isEnabled,
        isFalse,
      );
    });

    test('builds endpoints and validates URLs', () {
      expect(
        settings.endpoint('grades/details').toString(),
        'https://vtop.example.com/v1/grades/details',
      );
      expect(VtopServerSettings.validateUrl(''), isNull);
      expect(
        VtopServerSettings.validateUrl('https://vtop.example.com'),
        isNull,
      );
      expect(VtopServerSettings.validateUrl('http://10.0.2.2:8080'), isNull);
      expect(
        VtopServerSettings.validateUrl('http://vtop.example.com'),
        isNotNull,
      );
      expect(VtopServerSettings.validateUrl('vtop.example.com'), isNotNull);
    });
  });

  group('checkVtopServer', () {
    http.Client server({int probeStatus = 400}) => MockClient((request) async {
      if (request.url.path == '/health') {
        return http.Response('{"status":"ok","version":"0.1.0"}', 200);
      }
      expect(request.url.path, '/v1/semesters');
      return http.Response('{}', probeStatus);
    });

    test('reports the version when the key is accepted', () async {
      final result = await checkVtopServer(settings, httpClient: server());
      expect(result.isOk, isTrue);
      expect(result.version, '0.1.0');
    });

    test('reports a server that needs a key when none is set', () async {
      final result = await checkVtopServer(
        const VtopServerSettings(url: 'https://vtop.example.com', apiKey: ''),
        httpClient: server(probeStatus: 401),
      );
      expect(result.error, 'This server needs an API key.');
    });

    test('reports a rejected key', () async {
      final result = await checkVtopServer(
        settings,
        httpClient: server(probeStatus: 401),
      );
      expect(result.isOk, isFalse);
      expect(result.error, contains('API key'));
    });

    test('reports a URL that is not a vtop-server', () async {
      final result = await checkVtopServer(
        settings,
        httpClient: MockClient((_) async => http.Response('<html>', 200)),
      );
      expect(result.isOk, isFalse);
    });
  });

  group('prefetch', () {
    Map<String, Object?> refreshReply() => {
      'attendance': {'data': rustSnapshot('attendance')},
      'marks': {
        'error': {'code': 'vtop_error', 'message': 'VTOP broke'},
      },
    };

    test('later fetches use the batch instead of calling again', () async {
      final paths = <String>[];
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient((request) async {
          paths.add(request.url.path);
          final body = jsonDecode(request.body) as Map<String, dynamic>;
          if (request.url.path == '/v1/refresh') {
            expect(body['include'], ['attendance', 'marks']);
            return http.Response(jsonEncode(refreshReply()), 200);
          }
          return http.Response(jsonEncode(rustSnapshot('attendance')), 200);
        }),
      );

      await backend.prefetch('AP2026271', {
        VtopPage.attendance,
        VtopPage.marks,
      });
      final attendance = await backend.attendance('AP2026271');
      await expectLater(
        backend.marks('AP2026271'),
        throwsA(isA<VtopError_VtopServerError>()),
      );
      // Each prefetched page is used once; the next call goes to the server.
      await backend.attendance('AP2026271');

      expect(attendance.records, hasLength(3));
      expect(paths, ['/v1/refresh', '/v1/attendance']);
    });

    test('a prefetched page for another semester is ignored', () async {
      final paths = <String>[];
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient((request) async {
          paths.add(request.url.path);
          if (request.url.path == '/v1/refresh') {
            return http.Response(jsonEncode(refreshReply()), 200);
          }
          return http.Response(jsonEncode(rustSnapshot('attendance')), 200);
        }),
      );

      await backend.prefetch('AP2026271', {VtopPage.attendance});
      await backend.attendance('OTHER');

      expect(paths, ['/v1/refresh', '/v1/attendance']);
    });

    test('a failed batch falls back to single fetches', () async {
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient((request) async {
          if (request.url.path == '/v1/refresh') {
            return http.Response('{"error":{"code":"timeout"}}', 504);
          }
          return http.Response(jsonEncode(rustSnapshot('attendance')), 200);
        }),
      );

      await backend.prefetch('S', {VtopPage.attendance});
      expect((await backend.attendance('S')).records, hasLength(3));
    });

    test('an expired session during the batch is reported', () async {
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
        httpClient: MockClient(
          (_) async =>
              http.Response('{"error":{"code":"session_expired"}}', 401),
        ),
      );
      await expectLater(
        backend.prefetch('S', {VtopPage.attendance}),
        throwsA(const VtopError.sessionExpired()),
      );
    });
  });

  test('prefetched course attendance is used once per course', () async {
    final paths = <String>[];
    Map<String, Object?> detail(String course, String type) => {
      'records': [],
      'semester_id': 'S',
      'update_time': 0,
      'course_id': course,
      'course_type': type,
    };
    final backend = RemoteVtopBackend(
      settings: settings,
      session: () async => const SessionState(cookies: 'JSESSIONID=abc'),
      httpClient: MockClient((request) async {
        paths.add(request.url.path);
        if (request.url.path == '/v1/refresh') {
          return http.Response(
            jsonEncode({
              'full_attendance': {
                'data': [detail('C1', 'ETH'), detail('C2', 'ELA')],
              },
            }),
            200,
          );
        }
        return http.Response(jsonEncode(detail('C3', 'TH')), 200);
      }),
    );

    await backend.prefetch('S', {VtopPage.fullAttendance});
    final first = await backend.fullAttendance(
      semesterId: 'S',
      courseId: 'C1',
      courseType: 'ETH',
    );
    await backend.fullAttendance(
      semesterId: 'S',
      courseId: 'C2',
      courseType: 'ELA',
    );
    // Not in the batch: fetched alone.
    await backend.fullAttendance(
      semesterId: 'S',
      courseId: 'C3',
      courseType: 'TH',
    );

    expect(first.courseId, 'C1');
    expect(paths, ['/v1/refresh', '/v1/attendance/full']);
  });

  group('session write-back', () {
    String header(Map<String, Object?> session) =>
        base64Url.encode(utf8.encode(jsonEncode(session))).replaceAll('=', '');

    Future<List<SessionState>> run(String returnedCookies) async {
      final changes = <SessionState>[];
      final backend = RemoteVtopBackend(
        settings: settings,
        session: () async => const SessionState(
          cookies: 'JSESSIONID=abc; SERVERID=s1',
          csrfToken: 'csrf',
          registrationNumber: '22BCE0000',
        ),
        onSessionChanged: (session) async => changes.add(session),
        httpClient: MockClient(
          (_) async => http.Response(
            jsonEncode(rustSnapshot('marks')),
            200,
            headers: {
              'x-vtop-session': header({
                'cookies': returnedCookies,
                'csrf_token': 'csrf',
                'registration_number': '22BCE0000',
              }),
            },
          ),
        ),
      );
      await backend.marks('S');
      return changes;
    }

    test('rotated cookies are handed back to the phone', () async {
      final changes = await run('JSESSIONID=new; SERVERID=s1');
      expect(changes.single.cookies, 'JSESSIONID=new; SERVERID=s1');
      expect(changes.single.csrfToken, 'csrf');
    });

    test('the same cookies in another order are not a change', () async {
      expect(await run('SERVERID=s1; JSESSIONID=abc'), isEmpty);
    });
  });
}
