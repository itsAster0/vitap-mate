import 'package:flutter/foundation.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
part 'global_async_queue_entity.freezed.dart';

@freezed
sealed class GlobalAsyncQueueEntity with _$GlobalAsyncQueueEntity {
  const factory GlobalAsyncQueueEntity({
    @Default({}) Map<String, Future> running,
  }) = _GlobalAsyncQueueEntity;
}
