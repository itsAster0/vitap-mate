// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'biometric_history_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(BiometricHistory)
final biometricHistoryProvider = BiometricHistoryFamily._();

final class BiometricHistoryProvider
    extends $AsyncNotifierProvider<BiometricHistory, BiometricData> {
  BiometricHistoryProvider._({
    required BiometricHistoryFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'biometricHistoryProvider',
         isAutoDispose: false,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$biometricHistoryHash();

  @override
  String toString() {
    return r'biometricHistoryProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  BiometricHistory create() => BiometricHistory();

  @override
  bool operator ==(Object other) {
    return other is BiometricHistoryProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$biometricHistoryHash() => r'7c4c9ffc8095b9680d4abfa8e696e443185bafba';

final class BiometricHistoryFamily extends $Family
    with
        $ClassFamilyOverride<
          BiometricHistory,
          AsyncValue<BiometricData>,
          BiometricData,
          FutureOr<BiometricData>,
          String
        > {
  BiometricHistoryFamily._()
    : super(
        retry: null,
        name: r'biometricHistoryProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: false,
      );

  BiometricHistoryProvider call(String date) =>
      BiometricHistoryProvider._(argument: date, from: this);

  @override
  String toString() => r'biometricHistoryProvider';
}

abstract class _$BiometricHistory extends $AsyncNotifier<BiometricData> {
  late final _$args = ref.$arg as String;
  String get date => _$args;

  FutureOr<BiometricData> build(String date);
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<BiometricData>, BiometricData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<BiometricData>, BiometricData>,
              AsyncValue<BiometricData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, () => build(_$args));
  }
}
