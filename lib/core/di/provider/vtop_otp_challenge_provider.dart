import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';
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
    required this.remainingSeconds,
    required this.message,
    this.errorMessage,
  });

  const VtopOtpChallengeState.idle()
    : isActive = false,
      isMinimized = false,
      isSubmitting = false,
      isResending = false,
      remainingSeconds = 0,
      message = '',
      errorMessage = null;

  final bool isActive;
  final bool isMinimized;
  final bool isSubmitting;
  final bool isResending;
  final int remainingSeconds;
  final String message;
  final String? errorMessage;

  VtopOtpChallengeState copyWith({
    bool? isActive,
    bool? isMinimized,
    bool? isSubmitting,
    bool? isResending,
    int? remainingSeconds,
    String? message,
    String? errorMessage,
    bool clearError = false,
  }) {
    return VtopOtpChallengeState(
      isActive: isActive ?? this.isActive,
      isMinimized: isMinimized ?? this.isMinimized,
      isSubmitting: isSubmitting ?? this.isSubmitting,
      isResending: isResending ?? this.isResending,
      remainingSeconds: remainingSeconds ?? this.remainingSeconds,
      message: message ?? this.message,
      errorMessage: clearError ? null : (errorMessage ?? this.errorMessage),
    );
  }
}

@Riverpod(keepAlive: true)
class VtopOtpChallenge extends _$VtopOtpChallenge {
  Timer? _ticker;
  Completer<void>? _completer;
  VtopClient? _client;

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
  }) {
    _client = client;
    if (state.isActive && _completer != null && !_completer!.isCompleted) {
      state = state.copyWith(
        isMinimized: false,
        message: message,
        clearError: true,
      );
      return _completer!.future;
    }

    _completer = Completer<void>();
    state = VtopOtpChallengeState(
      isActive: true,
      isMinimized: false,
      isSubmitting: false,
      isResending: false,
      remainingSeconds: _otpChallengeTimeout.inSeconds,
      message: message,
    );
    _startTicker();
    return _completer!.future;
  }

  void minimize() {
    if (!state.isActive) return;
    state = state.copyWith(isMinimized: true);
  }

  void reopen() {
    if (!state.isActive) return;
    state = state.copyWith(isMinimized: false);
  }

  void cancel() {
    _finishWithError(
      VtopError.authenticationFailed('OTP verification cancelled'),
    );
  }

  Future<void> submitOtp(String otp) async {
    if (!state.isActive || _client == null) return;
    final sanitizedOtp = otp.replaceAll(RegExp(r'\D'), '');
    if (sanitizedOtp.length != 6) {
      state = state.copyWith(
        errorMessage: 'Please enter a valid 6-digit OTP.',
        clearError: false,
      );
      return;
    }

    state = state.copyWith(isSubmitting: true, clearError: true);
    try {
      await vtopClientSubmitSecurityOtp(
        client: _client!,
        otpCode: sanitizedOtp,
      );
      _finishSuccess();
    } catch (error) {
      final message = _authMessage(error);
      if (_isInvalidOtp(message)) {
        state = state.copyWith(
          isSubmitting: false,
          isMinimized: false,
          errorMessage: message,
        );
        return;
      }
      _finishWithError(error);
    }
  }

  Future<void> resendOtp() async {
    if (!state.isActive || _client == null) return;
    if (state.remainingSeconds > 0 || state.isSubmitting || state.isResending) {
      return;
    }

    state = state.copyWith(isResending: true, clearError: true);
    try {
      await vtopClientResendSecurityOtp(client: _client!);
      state = state.copyWith(
        isResending: false,
        remainingSeconds: _otpChallengeTimeout.inSeconds,
        message: 'A new OTP has been sent to your registered email.',
        clearError: true,
      );
      _startTicker();
    } catch (error) {
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
        state = state.copyWith(
          remainingSeconds: 0,
          message:
              'OTP expired. Tap resend to get a new OTP and continue verification.',
          errorMessage: 'OTP expired. Please resend OTP.',
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
    _reset();
    if (completer != null && !completer.isCompleted) {
      completer.complete();
    }
  }

  void _finishWithError(Object error) {
    final completer = _completer;
    _reset();
    if (completer != null && !completer.isCompleted) {
      completer.completeError(error);
    }
  }

  void _reset() {
    _ticker?.cancel();
    _ticker = null;
    _client = null;
    _completer = null;
    state = const VtopOtpChallengeState.idle();
  }
}
