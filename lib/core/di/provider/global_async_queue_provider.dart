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
  // ignore: avoid_public_notifier_properties
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
    if (id.startsWith("vtop")) {
      final mainFutures =
          state.running.entries
              .where((entry) => entry.key.startsWith('vtop_login'))
              .map((entry) => entry.value)
              .toList();
      if (mainFutures.isNotEmpty) {
        log("waiting for the on start run code ", level: 400);
        await Future.wait(mainFutures);
      }
    }
    final future = task();
    final current1 = state.running;
    final before = {...current1, id: future};
    state = state.copyWith(running: before);
    _taskEvents.add(before.keys.toSet());
    log("added $id  to ${state.running}", level: 400);
    future.whenComplete(() {
      final before1 = state.running;
      log("removing $id from $before1", level: 400);
      final after = {...before1}..remove(id);
      state = state.copyWith(running: after);
      _taskEvents.add(after.keys.toSet());
      log("$state");
    });

    return future;
  }
}
