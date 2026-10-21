import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/vtop_backend/relogin_vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_queries.dart';
import 'package:vitapmate/core/vtop_backend/vtop_query.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Answers every query and action from a queue of results.
class _ScriptedBackend implements VtopBackend {
  _ScriptedBackend(this.results);

  final List<Object> results;
  int calls = 0;

  @override
  String get name => 'scripted';

  Future<T> _next<T>() async {
    calls++;
    final next = results.removeAt(0);
    if (next is VtopError) throw next;
    return next as T;
  }

  @override
  Future<T> fetch<T>(VtopQuery<T> query) => _next<T>();

  @override
  Future<T> run<T>(VtopAction<T> action) => _next<T>();

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

final _data = TimetableData(
  slots: const [],
  courses: const [],
  semesterId: 'S',
  updateTime: BigInt.zero,
);

void main() {
  test('signs in again and retries once on an expired session', () async {
    final inner = _ScriptedBackend([const VtopError.sessionExpired(), _data]);
    var logins = 0;
    final backend = ReloginVtopBackend(inner, relogin: () async => logins++);

    expect(await backend.fetch(VtopQueries.timetable('S')), _data);
    expect(logins, 1);
    expect(inner.calls, 2);
  });

  test('gives up after one retry', () async {
    final inner = _ScriptedBackend([
      const VtopError.sessionExpired(),
      const VtopError.sessionExpired(),
    ]);
    final backend = ReloginVtopBackend(inner, relogin: () async {});

    await expectLater(
      backend.fetch(VtopQueries.timetable('S')),
      throwsA(const VtopError.sessionExpired()),
    );
    expect(inner.calls, 2);
  });

  test('other errors are not retried', () async {
    final inner = _ScriptedBackend([const VtopError.networkError()]);
    var logins = 0;
    final backend = ReloginVtopBackend(inner, relogin: () async => logins++);

    await expectLater(
      backend.fetch(VtopQueries.timetable('S')),
      throwsA(const VtopError.networkError()),
    );
    expect(logins, 0);
  });

  test('simultaneous failures share one re-login', () async {
    final inner = _ScriptedBackend([
      const VtopError.sessionExpired(),
      const VtopError.sessionExpired(),
      const VtopError.sessionExpired(),
      _data,
      _data,
      _data,
    ]);
    var logins = 0;
    final release = Completer<void>();
    final backend = ReloginVtopBackend(
      inner,
      relogin: () async {
        logins++;
        await release.future;
      },
    );

    final results = Future.wait([
      backend.fetch(VtopQueries.timetable('S')),
      backend.fetch(VtopQueries.timetable('S')),
      backend.fetch(VtopQueries.timetable('S')),
    ]);
    await Future<void>.delayed(Duration.zero);
    release.complete();

    expect(await results, [_data, _data, _data]);
    expect(logins, 1);
  });

  test('an action is sent again only after session expired', () async {
    const action = VtopAction<String>(
      name: 'cancel_general_outing',
      args: {'leave_id': 'L1'},
      fromJson: _string,
      local: _unused,
    );
    final expired = _ScriptedBackend([const VtopError.sessionExpired(), 'ok']);
    var logins = 0;
    expect(
      await ReloginVtopBackend(
        expired,
        relogin: () async => logins++,
      ).run(action),
      'ok',
    );
    expect((expired.calls, logins), (2, 1));

    final timedOut = _ScriptedBackend([const VtopError.networkError(), 'ok']);
    await expectLater(
      ReloginVtopBackend(timedOut, relogin: () async {}).run(action),
      throwsA(const VtopError.networkError()),
    );
    expect(timedOut.calls, 1);
  });

  test('a failed re-login surfaces its own error', () async {
    final inner = _ScriptedBackend([const VtopError.sessionExpired()]);
    final backend = ReloginVtopBackend(
      inner,
      relogin: () async => throw const VtopError.invalidCredentials(),
    );

    await expectLater(
      backend.fetch(VtopQueries.timetable('S')),
      throwsA(const VtopError.invalidCredentials()),
    );
  });

  test('a rejected password is reported once and not retried', () async {
    final inner = _ScriptedBackend([const VtopError.invalidCredentials()]);
    var logins = 0, rejected = 0;
    final backend = ReloginVtopBackend(
      inner,
      relogin: () async => logins++,
      onCredentialsRejected: () async => rejected++,
    );
    await expectLater(
      backend.fetch(VtopQueries.timetable('S')),
      throwsA(isA<VtopError_InvalidCredentials>()),
    );
    expect((logins, rejected, inner.calls), (0, 1, 1));
  });

  test('a rejection while signing in again is reported too', () async {
    final inner = _ScriptedBackend([const VtopError.sessionExpired()]);
    var rejected = 0;
    final backend = ReloginVtopBackend(
      inner,
      relogin: () async => throw const VtopError.invalidCredentials(),
      onCredentialsRejected: () async => rejected++,
    );
    await expectLater(
      backend.fetch(VtopQueries.timetable('S')),
      throwsA(isA<VtopError_InvalidCredentials>()),
    );
    expect(rejected, 1);
  });
}

String _string(Map<String, dynamic> json) => '$json';

Future<String> _unused(Object client) => throw UnimplementedError();
