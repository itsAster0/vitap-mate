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
    r'e1a941cf7485768120d7093997727f448c67482f';

/// Gmail access is connected (own key or shared). Server sign-in needs it.

@ProviderFor(emailOtpReady)
final emailOtpReadyProvider = EmailOtpReadyProvider._();

/// Gmail access is connected (own key or shared). Server sign-in needs it.

final class EmailOtpReadyProvider
    extends $FunctionalProvider<AsyncValue<bool>, bool, FutureOr<bool>>
    with $FutureModifier<bool>, $FutureProvider<bool> {
  /// Gmail access is connected (own key or shared). Server sign-in needs it.
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

/// Some way of reading the OTP is set up: Gmail or a forwarding address.

@ProviderFor(anyEmailOtpReady)
final anyEmailOtpReadyProvider = AnyEmailOtpReadyProvider._();

/// Some way of reading the OTP is set up: Gmail or a forwarding address.

final class AnyEmailOtpReadyProvider
    extends $FunctionalProvider<AsyncValue<bool>, bool, FutureOr<bool>>
    with $FutureModifier<bool>, $FutureProvider<bool> {
  /// Some way of reading the OTP is set up: Gmail or a forwarding address.
  AnyEmailOtpReadyProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'anyEmailOtpReadyProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$anyEmailOtpReadyHash();

  @$internal
  @override
  $FutureProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $FutureProviderElement(pointer);

  @override
  FutureOr<bool> create(Ref ref) {
    return anyEmailOtpReady(ref);
  }
}

String _$anyEmailOtpReadyHash() => r'700de9671fbda540452f59fa17bea39cd7bf3084';

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
    r'2d08babcc7b13b70426d94c7b6e379d7d7341b13';
