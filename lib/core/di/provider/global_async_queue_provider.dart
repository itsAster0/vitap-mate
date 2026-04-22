import 'dart:async';
import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/entity/global_async_queue_entity.dart';
part 'global_async_queue_provider.g.dart';

abstract class AsyncQueue {
  Future<T> run<T>(String id, Future<T> Function() task);
}

@Riverpod(keepAlive: true)
class GlobalAsyncQueue extends _$GlobalAsyncQueue implements AsyncQueue {
  final _taskEvents = StreamController<Set<String>>.broadcast();
  Stream<Set<String>> get taskStream => _taskEvents.stream;
  @override
  GlobalAsyncQueueEntity build() {
    return const GlobalAsyncQueueEntity();
  }

  @override
  Future<T> run<T>(String id, Future<T> Function() task) async {
    final current = state.running;
    if (current.containsKey(id)) {
      log("already contains $id", level: 400);
      return current[id] as Future<T>;
    }
    final completer = Completer<T>();
    final future = completer.future;
    final before = {...current, id: future};
    state = state.copyWith(running: before);
    _taskEvents.add(before.keys.toSet());

    unawaited(() async {
      try {
        if (id.startsWith("vtop")) {
          final mainFutures = current.entries
              .where(
                (entry) => entry.key.startsWith('vtop_login') && entry.key != id,
              )
              .map((entry) => entry.value)
              .toList();
          if (mainFutures.isNotEmpty) {
            log("waiting for the on start run code ", level: 400);
            await Future.wait(mainFutures);
          }
        }
        completer.complete(await task());
      } catch (e, st) {
        completer.completeError(e, st);
      } finally {
        final before1 = state.running;
        final after = {...before1}..remove(id);
        state = state.copyWith(running: after);
        _taskEvents.add(after.keys.toSet());
        log("$state");
      }
    }());

    return future;
  }
}
