import 'dart:async';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
part 'msg_utils.g.dart';

@riverpod
@riverpod
Stream<(RecordModel, PocketBase)> getUserrecord(Ref ref, String id) async* {
  final pb = await ref.watch(pbProvider.future);

  final record = await pb.collection('users').getOne(id);
  yield (record, pb);

  final controller = StreamController<(RecordModel, PocketBase)>();
  if (!pb.authStore.isValid) {
    ref.onDispose(controller.close);
    return;
  }
  final unsub = pb.collection('users').subscribe(id, (e) async {
    if (e.action == 'update') {
      final updated = await pb.collection('users').getOne(id);
      if (!controller.isClosed) {
        controller.add((updated, pb));
      }
    }
  });
  ref.onDispose(() {
    pb.collection('users').unsubscribe(id);
    controller.close();
  });

  yield* controller.stream;
}
