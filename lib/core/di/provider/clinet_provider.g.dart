// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'clinet_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(VClient)
final vClientProvider = VClientProvider._();

final class VClientProvider
    extends $AsyncNotifierProvider<VClient, VtopClient> {
  VClientProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vClientProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vClientHash();

  @$internal
  @override
  VClient create() => VClient();
}

String _$vClientHash() => r'0b41d0a427951e7ba11b9b03f6cb1eb2c084c160';

abstract class _$VClient extends $AsyncNotifier<VtopClient> {
  FutureOr<VtopClient> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<VtopClient>, VtopClient>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<VtopClient>, VtopClient>,
              AsyncValue<VtopClient>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
