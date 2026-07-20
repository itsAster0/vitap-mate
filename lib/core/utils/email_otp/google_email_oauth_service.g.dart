// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'google_email_oauth_service.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(googleEmailOtpAuthService)
final googleEmailOtpAuthServiceProvider = GoogleEmailOtpAuthServiceProvider._();

final class GoogleEmailOtpAuthServiceProvider
    extends
        $FunctionalProvider<
          GoogleEmailOtpAuthService,
          GoogleEmailOtpAuthService,
          GoogleEmailOtpAuthService
        >
    with $Provider<GoogleEmailOtpAuthService> {
  GoogleEmailOtpAuthServiceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'googleEmailOtpAuthServiceProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$googleEmailOtpAuthServiceHash();

  @$internal
  @override
  $ProviderElement<GoogleEmailOtpAuthService> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  GoogleEmailOtpAuthService create(Ref ref) {
    return googleEmailOtpAuthService(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(GoogleEmailOtpAuthService value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<GoogleEmailOtpAuthService>(value),
    );
  }
}

String _$googleEmailOtpAuthServiceHash() =>
    r'a94e4fa9bed50570c708d0cbca56a69e0672b6e2';

@ProviderFor(emailOtpReady)
final emailOtpReadyProvider = EmailOtpReadyProvider._();

final class EmailOtpReadyProvider
    extends $FunctionalProvider<AsyncValue<bool>, bool, FutureOr<bool>>
    with $FutureModifier<bool>, $FutureProvider<bool> {
  EmailOtpReadyProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'emailOtpReadyProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$emailOtpReadyHash();

  @$internal
  @override
  $FutureProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $FutureProviderElement(pointer);

  @override
  FutureOr<bool> create(Ref ref) {
    return emailOtpReady(ref);
  }
}

String _$emailOtpReadyHash() => r'96c9d98273240a683539a76dd5442fa666c560b4';

@ProviderFor(emailOtpSetupNeeded)
final emailOtpSetupNeededProvider = EmailOtpSetupNeededProvider._();

final class EmailOtpSetupNeededProvider
    extends $FunctionalProvider<AsyncValue<bool>, bool, FutureOr<bool>>
    with $FutureModifier<bool>, $FutureProvider<bool> {
  EmailOtpSetupNeededProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'emailOtpSetupNeededProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$emailOtpSetupNeededHash();

  @$internal
  @override
  $FutureProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $FutureProviderElement(pointer);

  @override
  FutureOr<bool> create(Ref ref) {
    return emailOtpSetupNeeded(ref);
  }
}

String _$emailOtpSetupNeededHash() =>
    r'7ea8deebd58bdee00bef9b973938aa5d9aefa39d';
