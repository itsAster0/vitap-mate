// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'mail_forwarding_service.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(mailForwardingService)
final mailForwardingServiceProvider = MailForwardingServiceProvider._();

final class MailForwardingServiceProvider
    extends
        $FunctionalProvider<
          MailForwardingService,
          MailForwardingService,
          MailForwardingService
        >
    with $Provider<MailForwardingService> {
  MailForwardingServiceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'mailForwardingServiceProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$mailForwardingServiceHash();

  @$internal
  @override
  $ProviderElement<MailForwardingService> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  MailForwardingService create(Ref ref) {
    return mailForwardingService(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(MailForwardingService value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<MailForwardingService>(value),
    );
  }
}

String _$mailForwardingServiceHash() =>
    r'c3983049926b74b325ba0fff6210f144101617c3';

@ProviderFor(mailForwarding)
final mailForwardingProvider = MailForwardingProvider._();

final class MailForwardingProvider
    extends
        $FunctionalProvider<
          AsyncValue<MailForwarding?>,
          MailForwarding?,
          FutureOr<MailForwarding?>
        >
    with $FutureModifier<MailForwarding?>, $FutureProvider<MailForwarding?> {
  MailForwardingProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'mailForwardingProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$mailForwardingHash();

  @$internal
  @override
  $FutureProviderElement<MailForwarding?> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<MailForwarding?> create(Ref ref) {
    return mailForwarding(ref);
  }
}

String _$mailForwardingHash() => r'18a633301c34aa6e2491467233e9c13b6d533602';
