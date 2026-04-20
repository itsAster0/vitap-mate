import 'dart:async';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

part 'vtop_otp_challenge_provider.g.dart';

const _otpChallengeTimeout = Duration(minutes: 3);

class VtopOtpChallengeState {
  const VtopOtpChallengeState({
    required this.isActive,
    required this.isMinimized,
    required this.isSubmitting,
    required this.isResending,
    required this.isAutoFetchingEmail,
    required this.remainingSeconds,
    required this.message,
    this.autoFetchMessage,
    this.errorMessage,
  });

  const VtopOtpChallengeState.idle()
    : isActive = false,
      isMinimized = false,
      isSubmitting = false,
      isResending = false,
      isAutoFetchingEmail = false,
      remainingSeconds = 0,
      message = '',
      autoFetchMessage = null,
      errorMessage = null;

  final bool isActive;
  final bool isMinimized;
  final bool isSubmitting;
  final bool isResending;
  final bool isAutoFetchingEmail;
  final int remainingSeconds;
  final String message;
  final String? autoFetchMessage;
  final String? errorMessage;

  VtopOtpChallengeState copyWith({
    bool? isActive,
    bool? isMinimized,
    bool? isSubmitting,
    bool? isResending,
    bool? isAutoFetchingEmail,
    int? remainingSeconds,
    String? message,
    String? autoFetchMessage,
    String? errorMessage,
    bool clearAutoFetchMessage = false,
    bool clearError = false,
  }) {
    return VtopOtpChallengeState(
      isActive: isActive ?? this.isActive,
      isMinimized: isMinimized ?? this.isMinimized,
      isSubmitting: isSubmitting ?? this.isSubmitting,
      isResending: isResending ?? this.isResending,
      isAutoFetchingEmail: isAutoFetchingEmail ?? this.isAutoFetchingEmail,
      remainingSeconds: remainingSeconds ?? this.remainingSeconds,
      message: message ?? this.message,
      autoFetchMessage: clearAutoFetchMessage
          ? null
          : (autoFetchMessage ?? this.autoFetchMessage),
      errorMessage: clearError ? null : (errorMessage ?? this.errorMessage),
    );
  }
}

@Riverpod(keepAlive: true)
class VtopOtpChallenge extends _$VtopOtpChallenge {
  Timer? _ticker;
  Completer<void>? _completer;
  VtopClient? _client;
  DateTime? _otpRequiredAt;
  int _autoFetchRunId = 0;
  int _challengeCounter = 0;
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
    if (state.isActive && _completer != null && !_completer!.isCompleted) {
      AppLogger.instance.info(
        'client.otp',
        '$_logContext challenge already active; refreshing prompt state',
      );
      state = state.copyWith(
        isMinimized: false,
        message: message,
        isAutoFetchingEmail: canAutoFetchFromEmail,
        autoFetchMessage: canAutoFetchFromEmail
            ? 'Trying to get OTP from email...'
            : null,
        clearError: true,
      );
      return _completer!.future;
    }

    _completer = Completer<void>();
    AppLogger.instance.info(
      'client.otp',
      '$_logContext challenge started (emailAutofetch=$canAutoFetchFromEmail)',
    );
    state = VtopOtpChallengeState(
      isActive: true,
      isMinimized: canAutoFetchFromEmail,
      isSubmitting: false,
      isResending: false,
      isAutoFetchingEmail: canAutoFetchFromEmail,
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

  void minimize() {
    if (!state.isActive) return;
    state = state.copyWith(isMinimized: true);
  }

  void reopen() {
    if (!state.isActive) return;
    state = state.copyWith(isMinimized: false);
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
      state = state.copyWith(
        errorMessage: 'Please enter a valid 6-digit OTP.',
        clearError: false,
      );
      return;
    }

    AppLogger.instance.info('client.otp', '$_logContext submitting OTP code');
    state = state.copyWith(isSubmitting: true, clearError: true);
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
        state = state.copyWith(
          isSubmitting: false,
          isMinimized: false,
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

    state = state.copyWith(isResending: true, clearError: true);
    AppLogger.instance.info('client.otp', '$_logContext requesting OTP resend');
    try {
      await vtopClientResendSecurityOtp(client: _client!);
      _otpRequiredAt = DateTime.now().toUtc();
      AppLogger.instance.info('client.otp', '$_logContext resend completed');
      state = state.copyWith(
        isResending: false,
        remainingSeconds: _otpChallengeTimeout.inSeconds,
        message: 'A new OTP has been sent to your registered email.',
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
      state = state.copyWith(
        isResending: false,
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
        state = state.copyWith(
          remainingSeconds: 0,
          message:
              'OTP expired. Tap resend to get a new OTP and continue verification.',
          errorMessage: autoFetchPending
              ? null
              : 'OTP expired. Please resend OTP.',
          clearError: false,
        );
        return;
      }
      state = state.copyWith(remainingSeconds: next);
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

      state = state.copyWith(
        autoFetchMessage:
            'Trying to get OTP from email... (${attempts - attempt} attempts left)',
      );
      try {
        final otp = await oauth.fetchLatestOtpSince(sinceUtc: startedAt);
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
        state = state.copyWith(
          isAutoFetchingEmail: false,
          isMinimized: false,
          autoFetchMessage: null,
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
    state = state.copyWith(
      isAutoFetchingEmail: false,
      isMinimized: false,
      autoFetchMessage: null,
      message: 'Could not find OTP in email. Enter it manually to continue.',
      clearError: true,
    );
  }

  bool _shouldContinueAutoFetch(int runId) {
    return runId == _autoFetchRunId && state.isActive && _client != null;
  }
}
