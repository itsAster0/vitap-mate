// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'controller.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(BackgroundSync)
final backgroundSyncProvider = BackgroundSyncProvider._();

final class BackgroundSyncProvider
    extends $AsyncNotifierProvider<BackgroundSync, SyncState> {
  BackgroundSyncProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'backgroundSyncProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$backgroundSyncHash();

  @$internal
  @override
  BackgroundSync create() => BackgroundSync();
}

String _$backgroundSyncHash() => r'8caa83eb6cde26fe8ea51fd5dd561432e9a1ea31';

abstract class _$BackgroundSync extends $AsyncNotifier<SyncState> {
  FutureOr<SyncState> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<SyncState>, SyncState>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<SyncState>, SyncState>,
              AsyncValue<SyncState>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
