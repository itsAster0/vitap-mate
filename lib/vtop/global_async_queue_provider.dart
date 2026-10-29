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

  final _startedAt = <String, DateTime>{};

  /// When the running task [id] was queued, for the status line.
  DateTime? startedAt(String id) => _startedAt[id];

  /// Whether [id] is held back by a sign-in under way: every VTOP call waits
  /// for one (see [run]).
  bool isWaiting(String id) =>
      id.startsWith('vtop_') &&
      !id.startsWith('vtop_login') &&
      state.running.keys.any((key) => key.startsWith('vtop_login'));

  @override
  GlobalAsyncQueueEntity build() => const GlobalAsyncQueueEntity();

  @override
  Future<T> run<T>(String id, Future<T> Function() task) async {
    final current = state.running;
    if (current.containsKey(id)) {
      Log.debug('vtop.queue', '$id already running; joining it');
      return current[id] as Future<T>;
    }
    final completer = Completer<T>();
    final future = completer.future;
    final running = {...current, id: future};
    _startedAt[id] = DateTime.now();
    state = state.copyWith(running: running);
    _taskEvents.add(running.keys.toSet());

    unawaited(() async {
      try {
        // Each task is a flow: its log lines, server requests and Rust
        // calls carry one id.
        final result = await flow(id, source: 'vtop.queue', () async {
          if (id.startsWith('vtop')) {
            // Every VTOP call waits for a sign-in already under way.
            final mainFutures = [
              for (final MapEntry(:key, :value) in current.entries)
                if (key.startsWith('vtop_login') && key != id) value,
            ];
            if (mainFutures.isNotEmpty) {
              Log.info('vtop.queue', '$id waiting for sign-in');
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
          return result;
        });
        completer.complete(result);
      } catch (e, st) {
        completer.completeError(e, st);
      } finally {
        final after = {...state.running}..remove(id);
        _startedAt.remove(id);
        state = state.copyWith(running: after);
        _taskEvents.add(after.keys.toSet());
      }
    }());

    return future;
  }
}
