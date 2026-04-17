// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'global_async_queue_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(GlobalAsyncQueue)
final globalAsyncQueueProvider = GlobalAsyncQueueProvider._();

final class GlobalAsyncQueueProvider
    extends $NotifierProvider<GlobalAsyncQueue, GlobalAsyncQueueEntity> {
  GlobalAsyncQueueProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'globalAsyncQueueProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$globalAsyncQueueHash();

  @$internal
  @override
  GlobalAsyncQueue create() => GlobalAsyncQueue();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(GlobalAsyncQueueEntity value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<GlobalAsyncQueueEntity>(value),
    );
  }
}

String _$globalAsyncQueueHash() => r'1842eb70b7871ca8f5cd27e7424e2868b6ef45b8';

abstract class _$GlobalAsyncQueue extends $Notifier<GlobalAsyncQueueEntity> {
  GlobalAsyncQueueEntity build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref as $Ref<GlobalAsyncQueueEntity, GlobalAsyncQueueEntity>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<GlobalAsyncQueueEntity, GlobalAsyncQueueEntity>,
              GlobalAsyncQueueEntity,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
