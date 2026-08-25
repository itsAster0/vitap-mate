// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'biometric_history_repository.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(biometricHistoryRepository)
final biometricHistoryRepositoryProvider = BiometricHistoryRepositoryFamily._();

final class BiometricHistoryRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<BiometricHistoryRepository>,
          BiometricHistoryRepository,
          FutureOr<BiometricHistoryRepository>
        >
    with
        $FutureModifier<BiometricHistoryRepository>,
        $FutureProvider<BiometricHistoryRepository> {
  BiometricHistoryRepositoryProvider._({
    required BiometricHistoryRepositoryFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'biometricHistoryRepositoryProvider',
         isAutoDispose: false,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$biometricHistoryRepositoryHash();

  @override
  String toString() {
    return r'biometricHistoryRepositoryProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  $FutureProviderElement<BiometricHistoryRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<BiometricHistoryRepository> create(Ref ref) {
    final argument = this.argument as String;
    return biometricHistoryRepository(ref, argument);
  }

  @override
  bool operator ==(Object other) {
    return other is BiometricHistoryRepositoryProvider &&
        other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$biometricHistoryRepositoryHash() =>
    r'a42af88c71403b9a9ce1c760a15a0ecdd4fabaa2';

final class BiometricHistoryRepositoryFamily extends $Family
    with
        $FunctionalFamilyOverride<
          FutureOr<BiometricHistoryRepository>,
          String
        > {
  BiometricHistoryRepositoryFamily._()
    : super(
        retry: null,
        name: r'biometricHistoryRepositoryProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: false,
      );

  BiometricHistoryRepositoryProvider call(String date) =>
      BiometricHistoryRepositoryProvider._(argument: date, from: this);

  @override
  String toString() => r'biometricHistoryRepositoryProvider';
}
