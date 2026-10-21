// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vtop_backend_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning
/// Whether a re-login may show the OTP prompt. Headless containers
/// (background sync, the FCM cookie bridge) override this to false.

@ProviderFor(vtopLoginPromptAllowed)
final vtopLoginPromptAllowedProvider = VtopLoginPromptAllowedProvider._();

/// Whether a re-login may show the OTP prompt. Headless containers
/// (background sync, the FCM cookie bridge) override this to false.

final class VtopLoginPromptAllowedProvider
    extends $FunctionalProvider<bool, bool, bool>
    with $Provider<bool> {
  /// Whether a re-login may show the OTP prompt. Headless containers
  /// (background sync, the FCM cookie bridge) override this to false.
  VtopLoginPromptAllowedProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopLoginPromptAllowedProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopLoginPromptAllowedHash();

  @$internal
  @override
  $ProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  bool create(Ref ref) {
    return vtopLoginPromptAllowed(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$vtopLoginPromptAllowedHash() =>
    r'6bc03c9f869e783d91d2f15c72f9f13820fd0b97';

@ProviderFor(vtopBackend)
final vtopBackendProvider = VtopBackendProvider._();

final class VtopBackendProvider
    extends $FunctionalProvider<VtopBackend, VtopBackend, VtopBackend>
    with $Provider<VtopBackend> {
  VtopBackendProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopBackendProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopBackendHash();

  @$internal
  @override
  $ProviderElement<VtopBackend> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  VtopBackend create(Ref ref) {
    return vtopBackend(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(VtopBackend value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<VtopBackend>(value),
    );
  }
}

String _$vtopBackendHash() => r'dd8143cb5d767b27288bf9b2d8ba97888239342c';

/// Logs in on the device; the edge signs in on its own when it fetches.

@ProviderFor(vtopAuthenticator)
final vtopAuthenticatorProvider = VtopAuthenticatorProvider._();

/// Logs in on the device; the edge signs in on its own when it fetches.

final class VtopAuthenticatorProvider
    extends
        $FunctionalProvider<
          VtopAuthenticator,
          VtopAuthenticator,
          VtopAuthenticator
        >
    with $Provider<VtopAuthenticator> {
  /// Logs in on the device; the edge signs in on its own when it fetches.
  VtopAuthenticatorProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopAuthenticatorProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopAuthenticatorHash();

  @$internal
  @override
  $ProviderElement<VtopAuthenticator> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  VtopAuthenticator create(Ref ref) {
    return vtopAuthenticator(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(VtopAuthenticator value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<VtopAuthenticator>(value),
    );
  }
}

String _$vtopAuthenticatorHash() => r'c937bb644bb340d647d6c7ad4241010639fc8136';
