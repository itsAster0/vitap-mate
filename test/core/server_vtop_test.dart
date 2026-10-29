import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/vtop/backend/server_json.dart';
import 'package:vitapmate/vtop/backend/server_vtop.dart';
import 'package:vitapmate/vtop/edge/server_status.dart' show EdgeIdentity;
import 'package:vitapmate/vtop/backend/vtop_backend.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/wire.dart';

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
  // The UDP backoff is shared by every api.
  tearDown(() => ServerVtopApi.wireOffUntil.value = null);
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
      PageDictionary? pages,
    }) => ServerVtopBackend(
      phone,
      session: () async => session,
      api: ServerVtopApi(
        client: client,
        baseUrl: 'https://edge.example/',
        pages: pages,
      ),
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
      expect(body.containsKey('dictionary'), isFalse);
    });

    http.Response packed(List<int> bytes) => http.Response.bytes(
      bytes,
      200,
      headers: {'content-type': packedPageType},
    );

    test('names its page dictionary and reads a page packed with it', () async {
      Map<String, dynamic>? body;
      final client = MockClient((request) async {
        body = jsonDecode(request.body) as Map<String, dynamic>;
        return packed([1, 2, 3]);
      });
      final pages = PageDictionary(
        id: () => 'd1',
        unpack: (name, bytes) {
          expect(name, 'semesters');
          expect(bytes, [1, 2, 3]);
          return jsonEncode(_fixture('semesters'));
        },
      );
      final phone = _Phone('unused');
      final data = await backend(
        client,
        phone,
        pages: pages,
      ).fetch(VtopQueries.semesters());
      expect(body!['dictionary'], 'd1');
      expect(data.semesters, isNotEmpty);
      expect(phone.asked, isEmpty);
    });

    test(
      'a packed page that does not unpack is fetched on the phone',
      () async {
        final client = MockClient((_) async => packed([9]));
        final pages = PageDictionary(
          id: () => 'd1',
          unpack: (_, _) => throw Exception('corrupt'),
        );
        final phone = _Phone(
          SemesterData(semesters: const [], updateTime: BigInt.zero),
        );
        await backend(
          client,
          phone,
          pages: pages,
        ).fetch(VtopQueries.semesters());
        expect(phone.asked, ['semesters']);
      },
    );

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

  group('ServerVtopApi over UDP', () {
    WirePage page(int status, Object json) => WirePage(
      status: status,
      packed: false,
      body: Uint8List.fromList(utf8.encode(jsonEncode(json))),
      elapsedMs: 12,
      handshake: false,
      wireBytes: 300,
    );

    /// An edge that answers every fetch with `{"data": {"via": "https"}}`.
    MockClient https(List<String> asked) => MockClient((request) async {
      asked.add(request.url.path);
      return http.Response('{"data":{"via":"https"}}', 200);
    });

    ServerVtopApi api(
      List<String> asked,
      Future<WirePage> Function() wire, {
      bool on = true,
    }) => ServerVtopApi(
      client: https(asked),
      baseUrl: 'https://edge.example',
      useWire: () async => on,
      wire:
          ({
            required sessionJson,
            required name,
            semester,
            required args,
            dictionary,
            account,
            secret = false,
            timeout,
          }) {
            asked.add('udp $name');
            expect(jsonDecode(sessionJson)['cookies'], 'JSESSIONID=a');
            return wire();
          },
    );

    test('pages come over UDP when the mode is on', () async {
      final asked = <String>[];
      final data = await api(
        asked,
        () async => page(200, {
          'data': {'via': 'udp'},
        }),
      ).fetch(session: _session, name: 'attendance');
      expect(data, {'via': 'udp'});
      expect(asked, ['udp attendance']);
    });

    test('the mode off leaves UDP alone', () async {
      final asked = <String>[];
      final data = await api(
        asked,
        () async => fail('no UDP'),
        on: false,
      ).fetch(session: _session, name: 'attendance');
      expect(data, {'via': 'https'});
      expect(asked, ['/v1/vtop/fetch']);
    });

    test(
      'an unreachable gateway falls back to HTTPS and is skipped after',
      () async {
        final asked = <String>[];
        final server = api(
          asked,
          () async => throw const WireFetchError.unreachable(),
        );
        expect(await server.fetch(session: _session, name: 'marks'), {
          'via': 'https',
        });
        expect(await server.fetch(session: _session, name: 'marks'), {
          'via': 'https',
        });
        expect(asked, ['udp marks', '/v1/vtop/fetch', '/v1/vtop/fetch']);
      },
    );

    test('a gateway older than the request is skipped for a while', () async {
      final asked = <String>[];
      final server = api(
        asked,
        () async => page(426, {
          'code': 'unsupported_version',
          'error': 'The gateway is older than the app.',
        }),
      );
      expect(await server.fetch(session: _session, name: 'marks'), {
        'via': 'https',
      });
      expect(await server.fetch(session: _session, name: 'marks'), {
        'via': 'https',
      });
      expect(asked, ['udp marks', '/v1/vtop/fetch', '/v1/vtop/fetch']);
    });

    test('a page too large for UDP comes over HTTPS', () async {
      final asked = <String>[];
      final data = await api(
        asked,
        () async => page(413, {'code': 'too_large', 'error': 'too large'}),
      ).fetch(session: _session, name: 'grade_history');
      expect(data, {'via': 'https'});
      expect(asked, ['udp grade_history', '/v1/vtop/fetch']);
    });

    test('VTOP refusals over UDP are the usual errors', () async {
      final asked = <String>[];
      await expectLater(
        api(
          asked,
          () async =>
              page(401, {'code': 'session_expired', 'error': 'expired'}),
        ).fetch(session: _session, name: 'attendance'),
        throwsA(const VtopError.sessionExpired()),
      );
    });

    test('a page that stalls is a timeout, not an HTTPS retry', () async {
      final asked = <String>[];
      await expectLater(
        api(
          asked,
          () async => throw const WireFetchError.timeout(),
        ).fetch(session: _session, name: 'attendance'),
        throwsA(isA<VtopError>()),
      );
      expect(asked, ['udp attendance']);
    });
  });

  group('sign-in over UDP', () {
    WirePage answer(int status, Object json) => WirePage(
      status: status,
      packed: false,
      body: Uint8List.fromList(utf8.encode(jsonEncode(json))),
      elapsedMs: 12,
      handshake: true,
      wireBytes: 300,
    );

    /// Requests sent over UDP: (name, secret, JSON body).
    ServerVtopApi api(
      List<(String, bool, Object?)> sent,
      Future<WirePage> Function() wire, {
      EdgeIdentity? identity = (
        appSecret: 'vta_x',
        registrationNumber: '23BCE7000',
      ),
    }) => ServerVtopApi(
      client: MockClient((_) async => fail('no HTTPS')),
      baseUrl: 'https://edge.example',
      useWire: () async => true,
      useServerSignIn: () async => true,
      identity: () async => identity,
      wire:
          ({
            required sessionJson,
            required name,
            semester,
            required args,
            dictionary,
            account,
            secret = false,
            timeout,
          }) {
            expect(sessionJson, isEmpty);
            sent.add((name, secret, jsonDecode(args['body']!)));
            return wire();
          },
    );

    test("the phone's own sign-in goes on a forward-secret session", () async {
      final sent = <(String, bool, Object?)>[];
      final outcome = await api(
        sent,
        () async => answer(200, {
          'status': 'ok',
          'session': {'cookies': 'JSESSIONID=s'},
        }),
      ).login('23BCE7000', 'pw');
      expect(outcome, isA<ServerSignedIn>());
      expect(outcome.session.cookies, 'JSESSIONID=s');
      final (name, secret, body) = sent.single;
      expect((name, secret), ('/v1/vtop/login', true));
      expect(body, {'username': '23BCE7000', 'password': 'pw'});
    });

    test('the server signs in with its saved password', () async {
      final sent = <(String, bool, Object?)>[];
      final session = await api(
        sent,
        () async => answer(200, {
          'status': 'ok',
          'session': {'cookies': 'JSESSIONID=s', 'csrf_token': 'c'},
        }),
      ).loginOnServer();
      expect((session?.cookies, session?.csrfToken), ('JSESSIONID=s', 'c'));
      final (name, secret, body) = sent.single;
      expect((name, secret), ('/v1/app/login', true));
      expect(body, {'registrationNumber': '23BCE7000', 'appSecret': 'vta_x'});
    });

    test('a session without its CSRF token is not taken', () async {
      final session = await api(
        [],
        () async => answer(200, {
          'status': 'ok',
          'session': {'cookies': 'JSESSIONID=s'},
        }),
      ).loginOnServer();
      expect(session, isNull);
    });

    test('without a saved password the phone signs in as usual', () async {
      final session = await api(
        [],
        () async => answer(409, {'code': 'no_saved_credentials'}),
      ).loginOnServer();
      expect(session, isNull);
    });

    test('an unlinked phone does not ask', () async {
      final sent = <(String, bool, Object?)>[];
      expect(
        await api(
          sent,
          () async => fail('no UDP'),
          identity: null,
        ).loginOnServer(),
        isNull,
      );
      expect(sent, isEmpty);
    });

    test('a rejected password is not tried again', () async {
      await expectLater(
        api(
          [],
          () async => answer(401, {'code': 'invalid_credentials'}),
        ).loginOnServer(),
        throwsA(const VtopError.invalidCredentials()),
      );
    });

    test('a sign-in that timed out is not tried over HTTPS', () async {
      await expectLater(
        api(
          [],
          () async => throw const WireFetchError.timeout(),
        ).login('U', 'pw'),
        throwsA(isA<VtopError_VtopUnavailable>()),
      );
    });
  });

  group('server sign-in in the fetch', () {
    ServerVtopApi api({
      required bool serverSignIn,
      required List<String?> sent,
      required List<SessionState> adopted,
    }) => ServerVtopApi(
      client: MockClient((_) async => fail('no HTTPS')),
      baseUrl: 'https://edge.example',
      useWire: () async => true,
      useServerSignIn: () async => serverSignIn,
      identity: () async =>
          (appSecret: 'vta_x', registrationNumber: '23BCE7000'),
      onSession: (session) async => adopted.add(session),
      wire:
          ({
            required sessionJson,
            required name,
            semester,
            required args,
            dictionary,
            account,
            secret = false,
            timeout,
          }) async {
            sent.add(account);
            return WirePage(
              status: 200,
              packed: false,
              body: Uint8List.fromList(utf8.encode('{"data":{"ok":true}}')),
              elapsedMs: 1,
              handshake: false,
              wireBytes: 1,
              session: account == null ? null : '{"cookies":"JSESSIONID=new"}',
            );
          },
    );

    test(
      'the account goes with the page; a renewed session is taken',
      () async {
        final sent = <String?>[];
        final adopted = <SessionState>[];
        final data = await api(
          serverSignIn: true,
          sent: sent,
          adopted: adopted,
        ).fetch(session: _session, name: 'marks');
        expect(data, {'ok': true});
        expect(jsonDecode(sent.single!), {
          'registrationNumber': '23BCE7000',
          'appSecret': 'vta_x',
        });
        expect(adopted.single.cookies, 'JSESSIONID=new');
      },
    );

    test('with server sign-in off nothing of the account goes', () async {
      final sent = <String?>[];
      final adopted = <SessionState>[];
      final server = api(serverSignIn: false, sent: sent, adopted: adopted);
      await server.fetch(session: _session, name: 'marks');
      expect(sent, [null]);
      expect(adopted, isEmpty);
      expect(await server.loginOnServer(), isNull);
      expect(sent, [null], reason: 'no sign-in by the server was asked');
    });
  });

  test('server sign-in goes over HTTPS when UDP mode is off', () async {
    final asked = <String>[];
    final api = ServerVtopApi(
      client: MockClient((request) async {
        asked.add(request.url.path);
        return http.Response(
          '{"status":"ok","session":{"cookies":"JSESSIONID=s","csrf_token":"c"}}',
          200,
        );
      }),
      baseUrl: 'https://edge.example',
      useServerSignIn: () async => true,
      identity: () async =>
          (appSecret: 'vta_x', registrationNumber: '23BCE7000'),
    );
    expect((await api.loginOnServer())?.cookies, 'JSESSIONID=s');
    expect(asked, ['/v1/app/login']);
  });
}
