// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vtop_users_utils.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(Vtopusersutils)
final vtopusersutilsProvider = VtopusersutilsProvider._();

final class VtopusersutilsProvider
    extends $NotifierProvider<Vtopusersutils, FlutterSecureStorage> {
  VtopusersutilsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopusersutilsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopusersutilsHash();

  @$internal
  @override
  Vtopusersutils create() => Vtopusersutils();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(FlutterSecureStorage value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<FlutterSecureStorage>(value),
    );
  }
}

String _$vtopusersutilsHash() => r'6117c04197b59242d0ee6996fe32cf557d4ef942';

abstract class _$Vtopusersutils extends $Notifier<FlutterSecureStorage> {
  FlutterSecureStorage build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<FlutterSecureStorage, FlutterSecureStorage>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<FlutterSecureStorage, FlutterSecureStorage>,
              FlutterSecureStorage,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}

@ProviderFor(allUsersProvider)
final allUsersProviderProvider = AllUsersProviderProvider._();

final class AllUsersProviderProvider
    extends
        $FunctionalProvider<
          AsyncValue<(List<VtopUserEntity>, String?)>,
          (List<VtopUserEntity>, String?),
          FutureOr<(List<VtopUserEntity>, String?)>
        >
    with
        $FutureModifier<(List<VtopUserEntity>, String?)>,
        $FutureProvider<(List<VtopUserEntity>, String?)> {
  AllUsersProviderProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'allUsersProviderProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$allUsersProviderHash();

  @$internal
  @override
  $FutureProviderElement<(List<VtopUserEntity>, String?)> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<(List<VtopUserEntity>, String?)> create(Ref ref) {
    return allUsersProvider(ref);
  }
}

String _$allUsersProviderHash() => r'33ab72898d4dfb5f8220887a838b5bd779417cee';
