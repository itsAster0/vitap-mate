// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_management.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(IsLoadingSems)
final isLoadingSemsProvider = IsLoadingSemsProvider._();

final class IsLoadingSemsProvider
    extends $NotifierProvider<IsLoadingSems, bool> {
  IsLoadingSemsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'isLoadingSemsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$isLoadingSemsHash();

  @$internal
  @override
  IsLoadingSems create() => IsLoadingSems();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$isLoadingSemsHash() => r'7d539b64bf8c9b1ad08699224969866a2056bedd';

abstract class _$IsLoadingSems extends $Notifier<bool> {
  bool build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<bool, bool>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<bool, bool>,
              bool,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
