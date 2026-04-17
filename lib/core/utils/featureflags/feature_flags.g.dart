// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'feature_flags.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(Gb)
final gbProvider = GbProvider._();

final class GbProvider extends $AsyncNotifierProvider<Gb, GrowthBookSDK> {
  GbProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'gbProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$gbHash();

  @$internal
  @override
  Gb create() => Gb();
}

String _$gbHash() => r'ef2cd4f243a23adcf41e1eacbc93a838673c4297';

abstract class _$Gb extends $AsyncNotifier<GrowthBookSDK> {
  FutureOr<GrowthBookSDK> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<GrowthBookSDK>, GrowthBookSDK>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<GrowthBookSDK>, GrowthBookSDK>,
              AsyncValue<GrowthBookSDK>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
