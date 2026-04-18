// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vtop_otp_challenge_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(VtopOtpChallenge)
final vtopOtpChallengeProvider = VtopOtpChallengeProvider._();

final class VtopOtpChallengeProvider
    extends $NotifierProvider<VtopOtpChallenge, VtopOtpChallengeState> {
  VtopOtpChallengeProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopOtpChallengeProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopOtpChallengeHash();

  @$internal
  @override
  VtopOtpChallenge create() => VtopOtpChallenge();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(VtopOtpChallengeState value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<VtopOtpChallengeState>(value),
    );
  }
}

String _$vtopOtpChallengeHash() => r'3b86114fa0194449d9ea09448c4474a3ac005b6f';

abstract class _$VtopOtpChallenge extends $Notifier<VtopOtpChallengeState> {
  VtopOtpChallengeState build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<VtopOtpChallengeState, VtopOtpChallengeState>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<VtopOtpChallengeState, VtopOtpChallengeState>,
              VtopOtpChallengeState,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
