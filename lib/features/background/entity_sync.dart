import 'package:freezed_annotation/freezed_annotation.dart';
part 'entity_sync.freezed.dart';
part 'entity_sync.g.dart';

@freezed
sealed class SyncState with _$SyncState {
  const factory SyncState({required Duration freq}) = _SyncState;
  factory SyncState.fromJson(Map<String, dynamic> json) =>
      _$SyncStateFromJson(json);
}
