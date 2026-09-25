import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/vtop_backend/relogin_vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Answers `timetable` from a queue of results; everything else is unused.
class _ScriptedBackend implements VtopBackend {
  _ScriptedBackend(this.results);

  final List<Object> results;
  int calls = 0;

  @override
  String get name => 'scripted';

  @override
  Future<TimetableData> timetable(String semesterId) async {
    calls++;
    final next = results.removeAt(0);
    if (next is VtopError) throw next;
    return next as TimetableData;
  }

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

    expect(await backend.timetable('S'), _data);
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
      backend.timetable('S'),
      throwsA(const VtopError.sessionExpired()),
    );
    expect(inner.calls, 2);
  });

  test('other errors are not retried', () async {
    final inner = _ScriptedBackend([const VtopError.networkError()]);
    var logins = 0;
    final backend = ReloginVtopBackend(inner, relogin: () async => logins++);

    await expectLater(
      backend.timetable('S'),
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
      backend.timetable('S'),
      backend.timetable('S'),
      backend.timetable('S'),
    ]);
    await Future<void>.delayed(Duration.zero);
    release.complete();

    expect(await results, [_data, _data, _data]);
    expect(logins, 1);
  });

  test('a failed re-login surfaces its own error', () async {
    final inner = _ScriptedBackend([const VtopError.sessionExpired()]);
    final backend = ReloginVtopBackend(
      inner,
      relogin: () async => throw const VtopError.invalidCredentials(),
    );

    await expectLater(
      backend.timetable('S'),
      throwsA(const VtopError.invalidCredentials()),
    );
  });
}
