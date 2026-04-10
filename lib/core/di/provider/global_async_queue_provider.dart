import 'dart:async';
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
      return current[id] as Future<T>;
    }
    if (id.startsWith("vtop")) {
      final mainFutures =
          state.running.entries
              .where((entry) => entry.key.startsWith('vtop_login'))
              .map((entry) => entry.value)
              .toList();
      if (mainFutures.isNotEmpty) {
        await Future.wait(mainFutures);
      }
    }
    late final Future<T> future;
    future = Future<T>.sync(task).whenComplete(() {
      final before1 = state.running;
      final after = {...before1}..remove(id);
      state = state.copyWith(running: after);
      _taskEvents.add(after.keys.toSet());
    });
    final before = {...state.running, id: future};
    state = state.copyWith(running: before);
    _taskEvents.add(before.keys.toSet());

    return future;
  }
}
