import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';

void main() {
  late ProviderContainer container;
  late GlobalAsyncQueue queue;
  late int signals;
  late StreamSubscription<void> subscription;
  setUp(() {
    container = ProviderContainer();
    queue = container.read(globalAsyncQueueProvider.notifier);
    signals = 0;
    subscription = queue.quickFetches.listen((_) => signals++);
  });
  tearDown(() async {
    await subscription.cancel();
    container.dispose();
  });

  test('a quick successful VTOP fetch signals a good connection', () async {
    await queue.run('vtop_attendance_AP1', () async => 1);
    await pumpEventQueue();
    expect(signals, 1);
  });

  test('slow, failed, login and storage tasks do not signal', () async {
    await queue.run('vtop_marks_AP1', () async {
      await Future<void>.delayed(
        GlobalAsyncQueue.quickFetchLimit + const Duration(milliseconds: 50),
      );
      return 1;
    });
    await expectLater(
      queue.run<int>('vtop_timetable_AP1', () async => throw StateError('x')),
      throwsStateError,
    );
    await queue.run('vtop_login_user', () async => 1);
    await queue.run('toStorage_marks', () async => 1);
    await pumpEventQueue();
    expect(signals, 0);
  });
}
