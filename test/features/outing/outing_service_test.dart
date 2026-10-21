import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';
import 'package:vitapmate/core/vtop_backend/vtop_queries.dart';
import 'package:vitapmate/features/outing/data/outing_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

import '../../support/recording_backend.dart';

void main() {
  ({ProviderContainer container, RecordingBackend backend}) rig(
    Object Function(String key) answer,
  ) {
    final backend = RecordingBackend(answer);
    final container = ProviderContainer(
      overrides: [vtopBackendProvider.overrideWithValue(backend)],
      retry: (_, _) => null,
    );
    addTearDown(container.dispose);
    return (container: container, backend: backend);
  }

  test('outing lists go through the backend by key, unsaved', () async {
    final r = rig((key) => throw StateError(key));
    final service = r.container.read(outingServiceProvider);
    await expectLater(service.general(), throwsStateError);
    await expectLater(service.weekend(), throwsStateError);
    expect(r.backend.fetched.map((q) => q.key), [
      'general_outing',
      'weekend_outing',
    ]);
  });

  test('the pass PDF comes back as bytes from base64', () async {
    final query = VtopQueries.outingPass(weekend: true, passId: 'P1');
    expect(query.key, 'outing_pass?kind=weekend&pass_id=P1');
    expect(
      query.fromJson({
        'pdf': base64.encode([1, 2, 3]),
      }),
      [1, 2, 3],
    );
  });

  test('apply goes through backend.run with the action key', () async {
    final r = rig(
      (key) => const OutingApplyResult(applied: true, message: 'ok'),
    );
    await r.container
        .read(outingServiceProvider)
        .applyGeneral(
          place: 'Home',
          purpose: 'Trip',
          leaving: DateTime(2026, 10, 21, 9, 5),
          returning: DateTime(2026, 10, 22, 18, 30),
          formatDate: (d) => '${d.day}-Oct-2026',
        );
    expect(r.backend.ran.map((a) => a.key), [
      'apply_general_outing?in_date=22-Oct-2026&in_time=18%3A30'
          '&out_date=21-Oct-2026&out_time=09%3A05&place=Home&purpose=Trip',
    ]);
    expect(r.backend.fetched, isEmpty);
  });

  test('cancel goes through backend.run with the action key', () async {
    final r = rig(
      (key) => const OutingCancelResult(cancelled: true, message: 'ok'),
    );
    await r.container
        .read(outingServiceProvider)
        .cancel(weekend: true, cancelId: 'B1');
    expect(r.backend.ran.single.key, 'cancel_weekend_outing?booking_id=B1');
  });

  test('a failed action reloads the outing lists', () async {
    final r = rig(
      (key) => key.startsWith('cancel')
          ? throw const VtopError.networkError()
          : throw StateError(key),
    );
    final sub = r.container.listen(generalOutingProvider.future, (_, _) {});
    addTearDown(sub.close);
    await expectLater(sub.read(), throwsStateError);
    await expectLater(
      r.container
          .read(outingServiceProvider)
          .cancel(weekend: false, cancelId: 'L1'),
      throwsA(isA<VtopError_NetworkError>()),
    );
    await expectLater(sub.read(), throwsStateError);
    expect(
      r.backend.fetched.where((q) => q.key == 'general_outing'),
      hasLength(2),
    );
  });
}
