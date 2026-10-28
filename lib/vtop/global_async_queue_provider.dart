import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/vtop/entity/global_async_queue_entity.dart';

part 'global_async_queue_provider.g.dart';

abstract class AsyncQueue {
  Future<T> run<T>(String id, Future<T> Function() task);
}

@Riverpod(keepAlive: true)
class GlobalAsyncQueue extends _$GlobalAsyncQueue implements AsyncQueue {
  final _taskEvents = StreamController<Set<String>>.broadcast();
  Stream<Set<String>> get taskStream => _taskEvents.stream;

  final _quickFetches = StreamController<void>.broadcast();

  /// Fires when a VTOP fetch succeeds within [quickFetchLimit]: the session
  /// is live and the network is good, so stale pages can be refreshed.
  Stream<void> get quickFetches => _quickFetches.stream;
  static const quickFetchLimit = Duration(seconds: 2);

  @override
  GlobalAsyncQueueEntity build() => const GlobalAsyncQueueEntity();

  @override
  Future<T> run<T>(String id, Future<T> Function() task) async {
    final current = state.running;
    if (current.containsKey(id)) {
      Log.info('vtop', "already contains $id");
      return current[id] as Future<T>;
    }
    final completer = Completer<T>();
    final future = completer.future;
    final running = {...current, id: future};
    state = state.copyWith(running: running);
    _taskEvents.add(running.keys.toSet());

    unawaited(() async {
      try {
        if (id.startsWith('vtop')) {
          // Every VTOP call waits for a sign-in already under way.
          final mainFutures = [
            for (final MapEntry(:key, :value) in current.entries)
              if (key.startsWith('vtop_login') && key != id) value,
          ];
          if (mainFutures.isNotEmpty) {
            Log.info('vtop', "waiting for the on start run code ");
            await Future.wait(mainFutures);
          }
        }
        final watch = Stopwatch()..start();
        final result = await task();
        if (id.startsWith('vtop_') &&
            !id.startsWith('vtop_login') &&
            watch.elapsed <= quickFetchLimit) {
          _quickFetches.add(null);
        }
        completer.complete(result);
      } catch (e, st) {
        completer.completeError(e, st);
      } finally {
        final after = {...state.running}..remove(id);
        state = state.copyWith(running: after);
        _taskEvents.add(after.keys.toSet());
        Log.info('vtop', "$state");
      }
    }());

    return future;
  }
}
