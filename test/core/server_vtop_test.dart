import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/core/providers/vtop_proxy.dart';
import 'package:vitapmate/core/vtop_backend/server_json.dart';
import 'package:vitapmate/core/vtop_backend/server_vtop.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_queries.dart';
import 'package:vitapmate/core/vtop_backend/vtop_query.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

const _session = SessionState(
  cookies: 'JSESSIONID=a',
  csrfToken: 'c',
  registrationNumber: '23BCE7000',
);

/// Answers every fetch with [answer] and keeps what was asked.
class _Phone implements VtopBackend {
  _Phone(this.answer);

  final Object answer;
  final asked = <String>[];

  @override
  String get name => 'phone';

  @override
  Future<T> fetch<T>(VtopQuery<T> query) async {
    asked.add(query.key);
    return answer as T;
  }

  @override
  Future<T> run<T>(VtopAction<T> action) async {
    asked.add('run ${action.key}');
    return answer as T;
  }
}

Map<String, dynamic> _fixture(String name) =>
    jsonDecode(
          File(
            'rust/vtop-core/tests/fixtures/$name.expected.json',
          ).readAsStringSync(),
        )
        as Map<String, dynamic>;

void main() {
  group("the engine's JSON reads as the app's types", () {
    // vtop-core's parser snapshots are its serde output, as the engine
    // sends it.
    final readers = <String, Object Function(Map<String, dynamic>)>{
      'attendance': AttendanceData.fromJson,
      'biometric': BiometricData.fromJson,
      'exam_schedule': ExamScheduleData.fromJson,
      'full_attendance': FullAttendanceData.fromJson,
      'grade_details': GradeDetailsData.fromJson,
      'grade_history': GradeHistoryData.fromJson,
      'grade_view': GradeViewData.fromJson,
      'marks': MarksData.fromJson,
      'semesters': SemesterData.fromJson,
      'timetable': TimetableData.fromJson,
    };
    for (final MapEntry(key: name, value: read) in readers.entries) {
      test(name, () {
        final json = rustJsonToDart(_fixture(name)) as Map<String, dynamic>;
        expect(() => read(json), returnsNormally);
      });
    }
  });

  test('a session goes to the engine and back unchanged', () {
    final session = SessionState(
      cookies: 'A=1',
      csrfToken: 'c',
      registrationNumber: 'R',
      otpIssuedAt: BigInt.from(5),
    );
    expect(sessionFromServerJson(sessionToServerJson(session)), session);
  });

  group('serverError', () {
    test("VTOP's refusals become the app's errors", () {
      expect(
        serverError(401, 'session_expired', ''),
        const VtopError.sessionExpired(),
      );
      expect(
        serverError(401, 'invalid_credentials', ''),
        const VtopError.invalidCredentials(),
      );
      expect(
        serverError(401, 'authentication_failed', 'Invalid OTP'),
        const VtopError.authenticationFailed('Invalid OTP'),
      );
      expect(
        serverError(502, 'vtop_unavailable', 'slow'),
        const VtopError.vtopUnavailable('slow'),
      );
    });

    test("the server's own trouble lets the phone try", () {
      for (final (status, code) in [
        (503, 'engine_unavailable'),
        (404, 'not_found'),
        (500, ''),
      ]) {
        expect(serverError(status, code, ''), isA<ServerVtopUnavailable>());
      }
    });
  });

  group('ServerVtopBackend', () {
    ServerVtopBackend backend(
      http.Client client,
      _Phone phone, {
      VtopServerMode mode = VtopServerMode.server,
      SessionState? session = _session,
    }) => ServerVtopBackend(
      phone,
      session: () async => session,
      api: ServerVtopApi(client: client, baseUrl: 'https://edge.example/'),
      mode: () async => mode,
    );

    test('reads a page through the server, parsed', () async {
      http.Request? sent;
      final client = MockClient((request) async {
        sent = request;
        return http.Response(jsonEncode({'data': _fixture('semesters')}), 200);
      });
      final phone = _Phone('unused');
      final data = await backend(
        client,
        phone,
      ).fetch(VtopQueries.semesters(semester: 'AP2025264'));
      expect(data.semesters, isNotEmpty);
      expect(phone.asked, isEmpty);
      expect(sent!.url.toString(), 'https://edge.example/v1/vtop/fetch');
      final body = jsonDecode(sent!.body) as Map<String, dynamic>;
      expect(body['name'], 'semesters');
      expect(body['semester'], 'AP2025264');
      expect(body['session'], {
        'cookies': 'JSESSIONID=a',
        'csrf_token': 'c',
        'registration_number': '23BCE7000',
      });
    });

    test('sends the arguments of a page that takes them', () async {
      Map<String, dynamic>? body;
      final client = MockClient((request) async {
        body = jsonDecode(request.body) as Map<String, dynamic>;
        return http.Response('{"code":"vtop_unavailable","error":"x"}', 502);
      });
      await expectLater(
        backend(
          client,
          _Phone('unused'),
        ).fetch(VtopQueries.gradeDetails('AP1', courseId: 'C1')),
        throwsA(const VtopError.vtopUnavailable('x')),
      );
      expect(body!['args'], {'course_id': 'C1'});
    });

    test(
      'an expired session is the usual expiry, so the app signs in',
      () async {
        final client = MockClient(
          (_) async => http.Response('{"code":"session_expired"}', 401),
        );
        await expectLater(
          backend(client, _Phone('unused')).fetch(VtopQueries.marks('AP1')),
          throwsA(const VtopError.sessionExpired()),
        );
      },
    );

    test('a phone not signed in yet signs in first', () async {
      final client = MockClient((_) async => fail('no request expected'));
      await expectLater(
        backend(
          client,
          _Phone('unused'),
          session: null,
        ).fetch(VtopQueries.marks('AP1')),
        throwsA(const VtopError.sessionExpired()),
      );
    });

    test('the phone fetches when the server cannot be reached', () async {
      final client = MockClient(
        (_) async => throw const SocketException('refused'),
      );
      final phone = _Phone(
        SemesterData(semesters: const [], updateTime: BigInt.zero),
      );
      await backend(client, phone).fetch(VtopQueries.semesters());
      expect(phone.asked, ['semesters']);
    });

    test(
      'other modes, phone-only pages and actions stay on the phone',
      () async {
        final client = MockClient((_) async => fail('no request expected'));
        final phone = _Phone(
          SemesterData(semesters: const [], updateTime: BigInt.zero),
        );
        await backend(
          client,
          phone,
          mode: VtopServerMode.compress,
        ).fetch(VtopQueries.semesters());
        final session = _Phone(_session);
        await backend(client, session).fetch(VtopQueries.vtopSession());
        final outing = _Phone(
          const OutingCancelResult(cancelled: true, message: ''),
        );
        await backend(
          client,
          outing,
        ).run(VtopActions.cancelGeneralOuting('L1'));
        phone.asked
          ..addAll(session.asked)
          ..addAll(outing.asked);
        expect(phone.asked, [
          'semesters',
          'vtop_session',
          'run cancel_general_outing?leave_id=L1',
        ]);
      },
    );
  });

  group('ServerVtopApi sign-in', () {
    test('an OTP request carries the half-done session back', () async {
      final client = MockClient(
        (_) async => http.Response(
          jsonEncode({
            'status': 'otp_required',
            'message': 'Enter the OTP',
            'issuedAt': 1700000000,
            'session': {'cookies': 'A=1', 'otp_issued_at': 1700000000},
          }),
          200,
        ),
      );
      final outcome = await ServerVtopApi(
        client: client,
        baseUrl: 'https://edge.example',
      ).login('23BCE7000', 'pw');
      expect(outcome, isA<ServerOtpRequired>());
      outcome as ServerOtpRequired;
      expect(outcome.issuedAt, BigInt.from(1700000000));
      expect(outcome.session.otpIssuedAt, BigInt.from(1700000000));
    });

    test('a wrong password is invalidCredentials', () async {
      final client = MockClient(
        (_) async => http.Response(
          '{"code":"invalid_credentials","error":"Invalid username or password"}',
          401,
        ),
      );
      await expectLater(
        ServerVtopApi(
          client: client,
          baseUrl: 'https://edge.example',
        ).login('23BCE7000', 'pw'),
        throwsA(const VtopError.invalidCredentials()),
      );
    });
  });
}
