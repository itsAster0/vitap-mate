// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'feature_flags.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(FeatureFlagsController)
final featureFlagsControllerProvider = FeatureFlagsControllerProvider._();

final class FeatureFlagsControllerProvider
    extends
        $AsyncNotifierProvider<
          FeatureFlagsController,
          FeatureFlagPodController
        > {
  FeatureFlagsControllerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'featureFlagsControllerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$featureFlagsControllerHash();

  @$internal
  @override
  FeatureFlagsController create() => FeatureFlagsController();
}

String _$featureFlagsControllerHash() =>
    r'2baf8c62a61eb0beb577eb8535904f7c86ae36b1';

abstract class _$FeatureFlagsController
    extends $AsyncNotifier<FeatureFlagPodController> {
  FutureOr<FeatureFlagPodController> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref
            as $Ref<
              AsyncValue<FeatureFlagPodController>,
              FeatureFlagPodController
            >;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<
                AsyncValue<FeatureFlagPodController>,
                FeatureFlagPodController
              >,
              AsyncValue<FeatureFlagPodController>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
