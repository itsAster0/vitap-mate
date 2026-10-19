import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';
import 'dart:async';
import 'package:flutter/widgets.dart' show AppLifecycleListener;
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/edge/edge_events.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/core/providers/on_device_mode.dart';

part 'vtop_otp_challenge_provider.g.dart';

const _otpChallengeTimeout = Duration(minutes: 3);

enum OtpChallengeActivity { waiting, submitting, resending, fetchingEmail }

enum OtpChallengePresentation { expanded, minimized }

/// Where the OTP is read from.
enum OtpSource {
  /// Typed by hand: no email auto-fill is set up.
  manual,

  /// Read from the student's Gmail inbox.
  gmail,

  /// Forwarded to the student's VITAP Mate address and pushed to the phone.
  forwarding,
}

/// What the prompt shows about the OTP being fetched.
class OtpFetchProgress {
  const OtpFetchProgress({
    required this.source,
    required this.requestedAt,
    this.account,
    this.checks = 0,
    this.lastCheckAt,
    this.gmailFallbackAvailable = false,
    this.gmailFallback = false,
    this.deleteAfterReading = false,
    this.problem,
  });

  /// After this long without a forwarded OTP, Gmail is read too.
  static const gmailFallbackAfter = Duration(seconds: 30);

  final OtpSource source;

  /// When VTOP was asked for the OTP (or asked again).
  final DateTime requestedAt;

  /// The Gmail address read, or the forwarding address.
  final String? account;

  /// Inbox checks so far: Gmail reads, or forwarded-mail pulls.
  final int checks;
  final DateTime? lastCheckAt;

  /// Forwarding only: Gmail is connected too, so it is read after
  /// [gmailFallbackAfter].
  final bool gmailFallbackAvailable;

  /// Forwarding only: Gmail is being read now as well.
  final bool gmailFallback;

  /// Gmail: the OTP mail goes to Trash once read.
  final bool deleteAfterReading;

  /// The last check failed; checking goes on.
  final String? problem;

  OtpFetchProgress checked({String? problem}) => OtpFetchProgress(
    source: source,
    requestedAt: requestedAt,
    account: account,
    checks: checks + 1,
    lastCheckAt: DateTime.now().toUtc(),
    gmailFallbackAvailable: gmailFallbackAvailable,
    gmailFallback: gmailFallback,
    deleteAfterReading: deleteAfterReading,
    problem: problem,
  );

  OtpFetchProgress withGmailFallback() => OtpFetchProgress(
    source: source,
    requestedAt: requestedAt,
    account: account,
    checks: checks,
    lastCheckAt: lastCheckAt,
    gmailFallbackAvailable: gmailFallbackAvailable,
    gmailFallback: true,
    deleteAfterReading: deleteAfterReading,
    problem: problem,
  );

  /// The same source, from a new request: counts start again.
  OtpFetchProgress restarted(DateTime requestedAt) => OtpFetchProgress(
    source: source,
    requestedAt: requestedAt,
    account: account,
    gmailFallbackAvailable: gmailFallbackAvailable,
    deleteAfterReading: deleteAfterReading,
  );
}

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
    OtpFetchProgress? progress,
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
      progress: progress,
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
  OtpFetchProgress? get progress;

  VtopOtpChallengeState update({
    OtpChallengeActivity? activity,
    OtpChallengePresentation? presentation,
    bool? canRetryEmailAutoFetch,
    int? remainingSeconds,
    String? message,
    String? autoFetchMessage,
    String? errorMessage,
    OtpFetchProgress? progress,
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
      progress: progress ?? this.progress,
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
  @override
  OtpFetchProgress? get progress => null;
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
    this.progress,
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
  @override
  final OtpFetchProgress? progress;
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

  /// The OTP comes by forwarded mail (pushed) rather than Gmail polling.
  bool _useMailForwarding = false;
  String _logContext = 'otp.flow';

  @override
  VtopOtpChallengeState build() {
    ref.onDispose(() {
      _autoFetchRunId++;
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
    _otpRequiredAt = otpRequiredAt?.toUtc() ?? DateTime.now().toUtc();
    _logContext = logContext ?? 'otp.flow#${++_challengeCounter}';
    final canAutoFetchFromEmail = await _canAutoFetchFromEmail();
    _emailAutoFetchAvailable = canAutoFetchFromEmail;
    final progress = await _progressFor(
      canAutoFetchFromEmail: canAutoFetchFromEmail,
      requestedAt: _otpRequiredAt!,
    );
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
        progress: progress,
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
      progress: progress,
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
      await ref
          .read(vtopAuthenticatorProvider)
          .submitOtp(_client!, sanitizedOtp);
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
      final requestedAt = DateTime.now().toUtc();
      await ref.read(vtopAuthenticatorProvider).resendOtp(_client!);
      _otpRequiredAt = requestedAt;
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
        progress: state.progress?.restarted(requestedAt),
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
    _useMailForwarding = false;
    _completer = null;
    state = const VtopOtpChallengeState.idle();
  }

  /// Forwarded mail comes first (pushed, so it's fastest); Gmail (own key
  /// or shared access) is the second choice.
  Future<bool> _canAutoFetchFromEmail() async {
    try {
      _useMailForwarding =
          !await isOnDeviceMode() &&
          await ref.read(mailForwardingServiceProvider).isReady();
      return _useMailForwarding || await _gmailReady();
    } catch (error, stackTrace) {
      AppLogger.instance.error(
        'client.otp',
        '$_logContext failed to evaluate email OTP autofetch availability: $error',
      );
      Zone.current.handleUncaughtError(error, stackTrace);
      return false;
    }
  }

  /// What the prompt shows for the source [_canAutoFetchFromEmail] chose.
  Future<OtpFetchProgress> _progressFor({
    required bool canAutoFetchFromEmail,
    required DateTime requestedAt,
  }) async {
    if (!canAutoFetchFromEmail) {
      return OtpFetchProgress(
        source: OtpSource.manual,
        requestedAt: requestedAt,
      );
    }
    try {
      if (_useMailForwarding) {
        final forwarding = await ref.read(mailForwardingServiceProvider).load();
        return OtpFetchProgress(
          source: OtpSource.forwarding,
          requestedAt: requestedAt,
          account: forwarding?.address,
          gmailFallbackAvailable: await _gmailReady(),
        );
      }
      final session = await ref
          .read(googleEmailOtpAuthServiceProvider)
          .loadSession();
      return OtpFetchProgress(
        source: OtpSource.gmail,
        requestedAt: requestedAt,
        account: session?.email,
        deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
      );
    } catch (_) {
      return OtpFetchProgress(
        source: _useMailForwarding ? OtpSource.forwarding : OtpSource.gmail,
        requestedAt: requestedAt,
      );
    }
  }

  /// Updates the progress while the prompt is up.
  void _updateProgress(
    OtpFetchProgress Function(OtpFetchProgress progress) change,
  ) {
    final progress = state.progress;
    if (!state.isActive || progress == null) return;
    state = state.update(progress: change(progress));
  }

  Future<bool> _gmailReady() async {
    final featureFlags = await ref.read(featureFlagsControllerProvider.future);
    return await featureFlags.isEnabled('2fa-email') &&
        await ref.read(googleEmailOtpAuthServiceProvider).isReady();
  }

  Future<void> _runEmailAutoFetch({required int runId}) async {
    if (_useMailForwarding) return _runMailForwardingAutoFetch(runId: runId);
    final otp = await _fetchFromGmail(runId: runId);
    if (otp != null) await submitOtp(otp);
  }

  /// Polls Gmail for the OTP until it turns up, the challenge ends or
  /// [stop] says another source found it. As a fallback ([stop] given),
  /// errors are only logged; on its own, they ask for the code by hand.
  Future<String?> _fetchFromGmail({
    required int runId,
    bool Function()? stop,
  }) async {
    bool keepGoing() =>
        _shouldContinueAutoFetch(runId) && !(stop?.call() ?? false);
    final oauth = ref.read(googleEmailOtpAuthServiceProvider);
    final startedAt = _otpRequiredAt!;
    AppLogger.instance.info(
      'client.otp',
      '$_logContext email autofetch started from ${startedAt.toIso8601String()}',
    );
    var delayMs = 1000;
    var attempt = 0;
    while (keepGoing()) {
      await Future<void>.delayed(Duration(milliseconds: delayMs));
      delayMs = (delayMs * 1.5).round().clamp(1000, 10000);
      attempt++;
      if (!keepGoing()) return null;

      state = state.update(autoFetchMessage: 'Waiting for OTP email...');
      try {
        final otp = await oauth.fetchLatestOtpSince(
          sinceUtc: startedAt,
          deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
        );
        if (!keepGoing()) return null;
        // Forwarded-mail pulls are the checks while Gmail is a fallback.
        if (stop == null) _updateProgress((p) => p.checked());
        if (otp != null && otp.isNotEmpty) {
          AppLogger.instance.info(
            'client.otp',
            '$_logContext email autofetch found an OTP on attempt $attempt',
          );
          return otp;
        }
      } catch (error, stackTrace) {
        AppLogger.instance.error(
          'client.otp',
          '$_logContext email autofetch failed on attempt $attempt: $error',
        );
        if (stop != null) {
          _updateProgress(
            (p) => p.checked(problem: 'Could not read Gmail too.'),
          );
          return null;
        }
        Zone.current.handleUncaughtError(error, stackTrace);
        if (!_shouldContinueAutoFetch(runId)) return null;
        state = state.update(
          activity: OtpChallengeActivity.waiting,
          canRetryEmailAutoFetch: true,
          presentation: OtpChallengePresentation.expanded,
          clearAutoFetchMessage: true,
          message:
              'Could not read OTP from email. Enter OTP manually to continue.',
          clearError: true,
        );
        return null;
      }
    }
    return null;
  }

  /// Waits for the forwarded OTP mail: over the edge's socket or FCM, and
  /// pulled every 2 s while no socket is open, in case a push is slow or
  /// held back, and on resume.
  /// With Gmail also connected, reads Gmail too if nothing came in 30 s.
  Future<void> _runMailForwardingAutoFetch({required int runId}) async {
    final service = ref.read(mailForwardingServiceProvider);
    final startedAt = _otpRequiredAt!;
    AppLogger.instance.info(
      'client.otp',
      '$_logContext waiting for a forwarded OTP from ${startedAt.toIso8601String()}',
    );
    state = state.update(autoFetchMessage: 'Waiting for the OTP email...');

    final found = Completer<String?>();
    void take(MailOtpPayload payload) {
      if (!found.isCompleted && payload.isOtpFor(startedAt)) {
        found.complete(payload.code);
      }
    }

    var pulling = false;
    Future<void> pull() async {
      if (found.isCompleted || pulling) return;
      pulling = true;
      try {
        final payload = await service.pullInbox();
        _updateProgress((p) => p.checked());
        if (payload != null) take(payload);
      } catch (error) {
        AppLogger.instance.warning(
          'client.otp',
          '$_logContext forwarded OTP pull failed: $error',
        );
        _updateProgress(
          (p) => p.checked(
            problem: 'Could not reach the VITAP Mate server; still trying.',
          ),
        );
      } finally {
        pulling = false;
      }
    }

    final subscription = mailOtpEvents.listen(take);
    final lifecycle = AppLifecycleListener(onResume: () => unawaited(pull()));
    final poll = Timer.periodic(const Duration(seconds: 2), (_) {
      if (!EdgeEventChannel.instance.connected.value) unawaited(pull());
    });
    // Ends the wait when the challenge is cancelled, expires or moves on.
    final watch = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!_shouldContinueAutoFetch(runId) && !found.isCompleted) {
        found.complete(null);
      }
    });
    // A push may have landed just before the challenge started listening.
    unawaited(pull());
    final gmailFallback = Timer(OtpFetchProgress.gmailFallbackAfter, () async {
      if (found.isCompleted || !await _gmailReady()) return;
      AppLogger.instance.info(
        'client.otp',
        '$_logContext no forwarded OTP yet; reading Gmail too',
      );
      _updateProgress((p) => p.withGmailFallback());
      final otp = await _fetchFromGmail(
        runId: runId,
        stop: () => found.isCompleted,
      );
      if (otp != null && !found.isCompleted) found.complete(otp);
    });

    final code = await found.future;
    await subscription.cancel();
    lifecycle.dispose();
    watch.cancel();
    gmailFallback.cancel();
    poll.cancel();
    if (code == null || !_shouldContinueAutoFetch(runId)) return;
    AppLogger.instance.info('client.otp', '$_logContext forwarded OTP arrived');
    await submitOtp(code);
  }

  bool _shouldContinueAutoFetch(int runId) {
    return runId == _autoFetchRunId &&
        state.isActive &&
        state.isAutoFetchingEmail &&
        state.remainingSeconds > 0 &&
        _client != null;
  }
}
