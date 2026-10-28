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

String _$vtopUsersHash() => r'7461c763b84ef7e8df8ad20805f573aeb895b26f';

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
