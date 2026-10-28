// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vtop_users.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(VtopUsers)
final vtopUsersProvider = VtopUsersProvider._();

final class VtopUsersProvider
    extends $NotifierProvider<VtopUsers, FlutterSecureStorage> {
  VtopUsersProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopUsersProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopUsersHash();

  @$internal
  @override
  VtopUsers create() => VtopUsers();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(FlutterSecureStorage value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<FlutterSecureStorage>(value),
    );
  }
}

String _$vtopUsersHash() => r'7211bbd832d0dc623ea1362eba52b4f37c6af8a5';

abstract class _$VtopUsers extends $Notifier<FlutterSecureStorage> {
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

String _$allUsersProviderHash() => r'dc932b3aea3a904d9ae03d0da7c75c8b16d560f8';
