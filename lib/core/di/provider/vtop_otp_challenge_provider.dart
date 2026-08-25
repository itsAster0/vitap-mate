import 'dart:async';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

part 'vtop_otp_challenge_provider.g.dart';

const _otpChallengeTimeout = Duration(minutes: 3);

enum OtpChallengeActivity { waiting, submitting, resending, fetchingEmail }

enum OtpChallengePresentation { expanded, minimized }

sealed class VtopOtpChallengeState {
  const VtopOtpChallengeState();

  const factory VtopOtpChallengeState.idle() = IdleOtpChallenge;

  factory VtopOtpChallengeState.active({
    required OtpChallengeActivity activity,
    required OtpChallengePresentation presentation,
    required int remainingSeconds,
    required String message,
    bool canRetryEmailAutoFetch = false,
    String? autoFetchMessage,
    String? errorMessage,
  }) {
    if (remainingSeconds < 0) {
      throw ArgumentError.value(
        remainingSeconds,
        'remainingSeconds',
        'must not be negative',
      );
    }
    if (canRetryEmailAutoFetch && activity != OtpChallengeActivity.waiting) {
      throw StateError('Email retry is only valid while waiting for an OTP.');
    }
    if (autoFetchMessage != null &&
        activity != OtpChallengeActivity.fetchingEmail) {
      throw StateError('An email fetch message requires email fetching.');
    }
    return ActiveOtpChallenge._(
      activity: activity,
      presentation: presentation,
      remainingSeconds: remainingSeconds,
      message: message,
      canRetryEmailAutoFetch: canRetryEmailAutoFetch,
      autoFetchMessage: autoFetchMessage,
      errorMessage: errorMessage,
    );
  }

  bool get isActive => this is ActiveOtpChallenge;
  bool get isMinimized =>
      this is ActiveOtpChallenge &&
      (this as ActiveOtpChallenge).presentation ==
          OtpChallengePresentation.minimized;
  bool get isSubmitting =>
      this is ActiveOtpChallenge &&
      (this as ActiveOtpChallenge).activity == OtpChallengeActivity.submitting;
  bool get isResending =>
      this is ActiveOtpChallenge &&
      (this as ActiveOtpChallenge).activity == OtpChallengeActivity.resending;
  bool get isAutoFetchingEmail =>
      this is ActiveOtpChallenge &&
      (this as ActiveOtpChallenge).activity ==
          OtpChallengeActivity.fetchingEmail;
  bool get canRetryEmailAutoFetch;
  int get remainingSeconds;
  String get message;
  String? get autoFetchMessage;
  String? get errorMessage;

  VtopOtpChallengeState update({
    OtpChallengeActivity? activity,
    OtpChallengePresentation? presentation,
    bool? canRetryEmailAutoFetch,
    int? remainingSeconds,
    String? message,
    String? autoFetchMessage,
    String? errorMessage,
    bool clearAutoFetchMessage = false,
    bool clearError = false,
  }) {
    final current = this;
    if (current is! ActiveOtpChallenge) {
      throw StateError('An idle OTP challenge cannot be updated.');
    }
    return VtopOtpChallengeState.active(
      activity: activity ?? current.activity,
      presentation: presentation ?? current.presentation,
      canRetryEmailAutoFetch:
          canRetryEmailAutoFetch ?? current.canRetryEmailAutoFetch,
      remainingSeconds: remainingSeconds ?? this.remainingSeconds,
      message: message ?? this.message,
      autoFetchMessage: clearAutoFetchMessage
          ? null
          : (autoFetchMessage ?? this.autoFetchMessage),
      errorMessage: clearError ? null : (errorMessage ?? this.errorMessage),
    );
  }
}

final class IdleOtpChallenge extends VtopOtpChallengeState {
  const IdleOtpChallenge();

  @override
  bool get canRetryEmailAutoFetch => false;
  @override
  int get remainingSeconds => 0;
  @override
  String get message => '';
  @override
  String? get autoFetchMessage => null;
  @override
  String? get errorMessage => null;
}

final class ActiveOtpChallenge extends VtopOtpChallengeState {
  const ActiveOtpChallenge._({
    required this.activity,
    required this.presentation,
    required this.canRetryEmailAutoFetch,
    required this.remainingSeconds,
    required this.message,
    this.autoFetchMessage,
    this.errorMessage,
  });

  final OtpChallengeActivity activity;
  final OtpChallengePresentation presentation;
  @override
  final bool canRetryEmailAutoFetch;
  @override
  final int remainingSeconds;
  @override
  final String message;
  @override
  final String? autoFetchMessage;
  @override
  final String? errorMessage;
}

@Riverpod(keepAlive: true)
class VtopOtpChallenge extends _$VtopOtpChallenge {
  Timer? _ticker;
  Completer<void>? _completer;
  VtopClient? _client;
  DateTime? _otpRequiredAt;
  int _autoFetchRunId = 0;
  int _challengeCounter = 0;
  bool _emailAutoFetchAvailable = false;
  String _logContext = 'otp.flow';

  @override
  VtopOtpChallengeState build() {
    ref.onDispose(() {
      _ticker?.cancel();
    });
    return const VtopOtpChallengeState.idle();
  }

  Future<void> requestOtp({
    required VtopClient client,
    String message =
        'Additional verification required. OTP sent to your registered email.',
    DateTime? otpRequiredAt,
    String? logContext,
  }) async {
    _client = client;
    _otpRequiredAt = otpRequiredAt?.toUtc();
    _logContext = logContext ?? 'otp.flow#${++_challengeCounter}';
    final canAutoFetchFromEmail = await _canAutoFetchFromEmail();
    _emailAutoFetchAvailable = canAutoFetchFromEmail;
    if (state.isActive && _completer != null && !_completer!.isCompleted) {
      AppLogger.instance.info(
        'client.otp',
        '$_logContext challenge already active; refreshing prompt state',
      );
      state = state.update(
        presentation: OtpChallengePresentation.expanded,
        message: message,
        activity: canAutoFetchFromEmail
            ? OtpChallengeActivity.fetchingEmail
            : OtpChallengeActivity.waiting,
        canRetryEmailAutoFetch: false,
        autoFetchMessage: canAutoFetchFromEmail
            ? 'Trying to get OTP from email...'
            : null,
        clearAutoFetchMessage: !canAutoFetchFromEmail,
        clearError: true,
      );
      return _completer!.future;
    }

    _completer = Completer<void>();
    AppLogger.instance.info(
      'client.otp',
      '$_logContext challenge started (emailAutofetch=$canAutoFetchFromEmail)',
    );
    state = VtopOtpChallengeState.active(
      presentation: canAutoFetchFromEmail
          ? OtpChallengePresentation.minimized
          : OtpChallengePresentation.expanded,
      activity: canAutoFetchFromEmail
          ? OtpChallengeActivity.fetchingEmail
          : OtpChallengeActivity.waiting,
      canRetryEmailAutoFetch: false,
      remainingSeconds: _otpChallengeTimeout.inSeconds,
      message: message,
      autoFetchMessage: canAutoFetchFromEmail
          ? 'Trying to get OTP from email...'
          : null,
    );
    _startTicker();
    if (canAutoFetchFromEmail) {
      final runId = ++_autoFetchRunId;
      unawaited(_runEmailAutoFetch(runId: runId));
    }
    return _completer!.future;
  }

  Future<bool> canAutoFetchFromEmail() => _canAutoFetchFromEmail();

  void retryEmailAutoFetch() {
    if (!state.isActive ||
        _client == null ||
        !state.canRetryEmailAutoFetch ||
        state.isAutoFetchingEmail ||
        state.isSubmitting ||
        state.isResending ||
        state.remainingSeconds == 0) {
      return;
    }

    AppLogger.instance.info(
      'client.otp',
      '$_logContext retrying email autofetch',
    );
    state = state.update(
      activity: OtpChallengeActivity.fetchingEmail,
      canRetryEmailAutoFetch: false,
      message:
          'Trying to get OTP from email again. You can still enter it manually.',
      autoFetchMessage: 'Trying to get OTP from email...',
      clearError: true,
    );
    final runId = ++_autoFetchRunId;
    unawaited(_runEmailAutoFetch(runId: runId));
  }

  void minimize() {
    if (!state.isActive) return;
    state = state.update(presentation: OtpChallengePresentation.minimized);
  }

  void reopen() {
    if (!state.isActive) return;
    state = state.update(presentation: OtpChallengePresentation.expanded);
  }

  void cancel() {
    AppLogger.instance.warning(
      'client.otp',
      '$_logContext challenge cancelled',
    );
    _finishWithError(
      VtopError.authenticationFailed('OTP verification cancelled'),
    );
  }

  Future<void> submitOtp(String otp) async {
    if (!state.isActive || _client == null) return;
    final sanitizedOtp = otp.replaceAll(RegExp(r'\D'), '');
    if (sanitizedOtp.length != 6) {
      AppLogger.instance.warning(
        'client.otp',
        '$_logContext rejected OTP submit because the code was not 6 digits',
      );
      state = state.update(
        errorMessage: 'Please enter a valid 6-digit OTP.',
        clearError: false,
      );
      return;
    }

    AppLogger.instance.info('client.otp', '$_logContext submitting OTP code');
    state = state.update(
      activity: OtpChallengeActivity.submitting,
      canRetryEmailAutoFetch: false,
      clearAutoFetchMessage: true,
      clearError: true,
    );
    try {
      await vtopClientSubmitSecurityOtp(
        client: _client!,
        otpCode: sanitizedOtp,
      );
      AppLogger.instance.info('client.otp', '$_logContext OTP accepted');
      _finishSuccess();
    } catch (error) {
      final message = _authMessage(error);
      if (_isInvalidOtp(message)) {
        AppLogger.instance.warning(
          'client.otp',
          '$_logContext OTP rejected as invalid',
        );
        state = state.update(
          activity: OtpChallengeActivity.waiting,
          presentation: OtpChallengePresentation.expanded,
          errorMessage: message,
        );
        return;
      }
      AppLogger.instance.error(
        'client.otp',
        '$_logContext OTP submission failed: $error',
      );
      _finishWithError(error);
    }
  }

  Future<void> resendOtp() async {
    if (!state.isActive || _client == null) return;
    if (state.remainingSeconds > 0 || state.isSubmitting || state.isResending) {
      return;
    }

    state = state.update(
      activity: OtpChallengeActivity.resending,
      canRetryEmailAutoFetch: false,
      clearAutoFetchMessage: true,
      clearError: true,
    );
    AppLogger.instance.info('client.otp', '$_logContext requesting OTP resend');
    try {
      await vtopClientResendSecurityOtp(client: _client!);
      _otpRequiredAt = DateTime.now().toUtc();
      AppLogger.instance.info('client.otp', '$_logContext resend completed');
      state = state.update(
        activity: _emailAutoFetchAvailable
            ? OtpChallengeActivity.fetchingEmail
            : OtpChallengeActivity.waiting,
        remainingSeconds: _otpChallengeTimeout.inSeconds,
        message: 'A new OTP has been sent to your registered email.',
        autoFetchMessage: _emailAutoFetchAvailable
            ? 'Trying to get OTP from email...'
            : null,
        clearError: true,
      );
      _startTicker();
      if (state.isAutoFetchingEmail) {
        final runId = ++_autoFetchRunId;
        unawaited(_runEmailAutoFetch(runId: runId));
      }
    } catch (error) {
      AppLogger.instance.error(
        'client.otp',
        '$_logContext resend failed: $error',
      );
      state = state.update(
        activity: OtpChallengeActivity.waiting,
        errorMessage: _authMessage(error),
        clearError: false,
      );
    }
  }

  void _startTicker() {
    _ticker?.cancel();
    _ticker = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!state.isActive) return;
      final next = state.remainingSeconds - 1;
      if (next <= 0) {
        _ticker?.cancel();
        final autoFetchPending = state.isAutoFetchingEmail;
        state = state.update(
          activity: OtpChallengeActivity.waiting,
          remainingSeconds: 0,
          message:
              'OTP expired. Tap resend to get a new OTP and continue verification.',
          errorMessage: autoFetchPending
              ? null
              : 'OTP expired. Please resend OTP.',
          clearAutoFetchMessage: true,
          clearError: false,
        );
        return;
      }
      state = state.update(remainingSeconds: next);
    });
  }

  bool _isInvalidOtp(String message) {
    return message.toLowerCase().contains('invalid otp');
  }

  String _authMessage(Object error) {
    if (error is VtopError) {
      return error.maybeWhen(
        authenticationFailed: (message) {
          final trimmed = message.trim();
          return trimmed.isEmpty ? 'Authentication failed' : trimmed;
        },
        orElse: () => 'Authentication failed',
      );
    }
    return 'Authentication failed';
  }

  void _finishSuccess() {
    final completer = _completer;
    AppLogger.instance.info('client.otp', '$_logContext challenge completed');
    _reset();
    if (completer != null && !completer.isCompleted) {
      completer.complete();
    }
  }

  void _finishWithError(Object error) {
    final completer = _completer;
    AppLogger.instance.warning(
      'client.otp',
      '$_logContext challenge finished with error: $error',
    );
    _reset();
    if (completer != null && !completer.isCompleted) {
      completer.completeError(error);
    }
  }

  void _reset() {
    _autoFetchRunId++;
    _ticker?.cancel();
    _ticker = null;
    _client = null;
    _otpRequiredAt = null;
    _emailAutoFetchAvailable = false;
    _completer = null;
    state = const VtopOtpChallengeState.idle();
  }

  Future<bool> _canAutoFetchFromEmail() async {
    try {
      final featureFlags = await ref.read(
        featureFlagsControllerProvider.future,
      );
      if (!await featureFlags.isEnabled('2fa-email')) return false;
      final oauth = ref.read(googleEmailOtpAuthServiceProvider);
      return oauth.isReady();
    } catch (error, stackTrace) {
      AppLogger.instance.error(
        'client.otp',
        '$_logContext failed to evaluate email OTP autofetch availability: $error',
      );
      Zone.current.handleUncaughtError(error, stackTrace);
      return false;
    }
  }

  Future<void> _runEmailAutoFetch({required int runId}) async {
    final oauth = ref.read(googleEmailOtpAuthServiceProvider);
    final startedAt =
        _otpRequiredAt ?? DateTime.now().subtract(Duration(seconds: 2)).toUtc();
    AppLogger.instance.info(
      'client.otp',
      '$_logContext email autofetch started from ${startedAt.toIso8601String()}',
    );
    final attempts = 8;
    for (var attempt = 0; attempt < attempts; attempt++) {
      if (!_shouldContinueAutoFetch(runId)) return;
      if (!state.isAutoFetchingEmail) return;
      if (state.isSubmitting) return;

      state = state.update(
        autoFetchMessage:
            'Trying to get OTP from email... (${attempts - attempt} attempts left)',
      );
      try {
        final otp = await oauth.fetchLatestOtpSince(
          sinceUtc: startedAt,
          deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
        );
        if (otp != null && otp.isNotEmpty) {
          AppLogger.instance.info(
            'client.otp',
            '$_logContext email autofetch found an OTP on attempt ${attempt + 1} of $attempts',
          );
          await submitOtp(otp);
          return;
        }
      } catch (error, stackTrace) {
        AppLogger.instance.error(
          'client.otp',
          '$_logContext email autofetch failed on attempt ${attempt + 1} of $attempts: $error',
        );
        Zone.current.handleUncaughtError(error, stackTrace);
        if (!_shouldContinueAutoFetch(runId)) return;
        state = state.update(
          activity: OtpChallengeActivity.waiting,
          canRetryEmailAutoFetch: true,
          presentation: OtpChallengePresentation.expanded,
          clearAutoFetchMessage: true,
          message:
              'Could not read OTP from email. Enter OTP manually to continue.',
          clearError: true,
        );
        return;
      }
      await Future<void>.delayed(const Duration(seconds: 1));
    }

    if (!_shouldContinueAutoFetch(runId)) return;
    AppLogger.instance.warning(
      'client.otp',
      '$_logContext email autofetch exhausted all attempts without finding an OTP',
    );
    state = state.update(
      activity: OtpChallengeActivity.waiting,
      canRetryEmailAutoFetch: true,
      presentation: OtpChallengePresentation.expanded,
      clearAutoFetchMessage: true,
      message: 'Could not find OTP in email. Enter it manually to continue.',
      clearError: true,
    );
  }

  bool _shouldContinueAutoFetch(int runId) {
    return runId == _autoFetchRunId && state.isActive && _client != null;
  }
}
