import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/global_async_queue_provider.dart';

void main() {
  test('a running task has a start time; a page waits on a sign-in', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final queue = container.read(globalAsyncQueueProvider.notifier);
    final login = Completer<void>();
    final done = queue.run('vtop_login_U', () => login.future);
    final page = queue.run('vtop_marks_S', () async => 1);

    expect(queue.startedAt('vtop_login_U'), isNotNull);
    expect(queue.isWaiting('vtop_marks_S'), isTrue);
    expect(queue.isWaiting('vtop_login_U'), isFalse);

    login.complete();
    await done;
    await page;
    expect(queue.startedAt('vtop_login_U'), isNull);
  });
}
