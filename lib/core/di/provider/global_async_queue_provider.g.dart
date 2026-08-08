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

String _$globalAsyncQueueHash() => r'ac6aae4a3e503131dfadc156d29e623d3def2b00';

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
