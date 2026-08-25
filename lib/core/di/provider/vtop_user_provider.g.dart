// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vtop_user_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(VtopUser)
final vtopUserProvider = VtopUserProvider._();

final class VtopUserProvider
    extends $AsyncNotifierProvider<VtopUser, VtopUserEntity> {
  VtopUserProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopUserProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopUserHash();

  @$internal
  @override
  VtopUser create() => VtopUser();
}

String _$vtopUserHash() => r'5d2ed3e4d460d42f0694dc1a960c9bb3271b7cb4';

abstract class _$VtopUser extends $AsyncNotifier<VtopUserEntity> {
  FutureOr<VtopUserEntity> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<VtopUserEntity>, VtopUserEntity>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<VtopUserEntity>, VtopUserEntity>,
              AsyncValue<VtopUserEntity>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
