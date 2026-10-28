import 'dart:async';

import 'package:flutter/widgets.dart' show AppLifecycleListener;
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/vtop/edge/edge_events.dart';
import 'package:vitapmate/vtop/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/vtop/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';

part 'vtop_otp_challenge_provider.g.dart';

const _otpChallengeTimeout = Duration(minutes: 3);

/// The prompt's message when VTOP has just asked for an OTP.
const defaultOtpMessage =
    'Additional verification required. OTP sent to your registered email.';

const _tryingEmail = 'Trying to get OTP from email...';
const _expiredMessage =
    'OTP expired. Tap resend to get a new OTP and continue verification.';

/// Shown after VTOP rejects a fetched OTP, often one from an earlier sign-in.
const keepTryingMessage =
    'That OTP did not work, probably from an earlier sign-in. Waiting for a newer one…';

/// Fetched codes VTOP may reject before the prompt asks by hand.
const _maxRejectedCodes = 3;

/// New OTPs sent on their own when one expires while auto-fill is on.
const _maxAutoResends = 2;

enum _Rejection { invalid, expired }

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

  OtpFetchProgress checked({String? problem}) => _with(
    requestedAt: requestedAt,
    checks: checks + 1,
    lastCheckAt: DateTime.now().toUtc(),
    gmailFallback: gmailFallback,
    problem: problem,
  );

  OtpFetchProgress withGmailFallback() => _with(
    requestedAt: requestedAt,
    checks: checks,
    lastCheckAt: lastCheckAt,
    gmailFallback: true,
    problem: problem,
  );

  /// The same source, from a new request: counts start again.
  OtpFetchProgress restarted(DateTime requestedAt) =>
      _with(requestedAt: requestedAt);

  /// The same source and account with these counts.
  OtpFetchProgress _with({
    required DateTime requestedAt,
    int checks = 0,
    DateTime? lastCheckAt,
    bool gmailFallback = false,
    String? problem,
  }) => OtpFetchProgress(
    source: source,
    requestedAt: requestedAt,
    account: account,
    checks: checks,
    lastCheckAt: lastCheckAt,
    gmailFallbackAvailable: gmailFallbackAvailable,
    gmailFallback: gmailFallback,
    deleteAfterReading: deleteAfterReading,
    problem: problem,
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
  bool get isMinimized => switch (this) {
    ActiveOtpChallenge(presentation: .minimized) => true,
    _ => false,
  };
  bool get isSubmitting => _activity == .submitting;
  bool get isResending => _activity == .resending;
  bool get isAutoFetchingEmail => _activity == .fetchingEmail;
  OtpChallengeActivity? get _activity => switch (this) {
    ActiveOtpChallenge(:final activity) => activity,
    IdleOtpChallenge() => null,
  };
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
    final current = switch (this) {
      ActiveOtpChallenge active => active,
      IdleOtpChallenge() => throw StateError(
        'An idle OTP challenge cannot be updated.',
      ),
    };
    return VtopOtpChallengeState.active(
      activity: activity ?? current.activity,
      presentation: presentation ?? current.presentation,
      canRetryEmailAutoFetch:
          canRetryEmailAutoFetch ?? current.canRetryEmailAutoFetch,
      remainingSeconds: remainingSeconds ?? current.remainingSeconds,
      message: message ?? current.message,
      autoFetchMessage: clearAutoFetchMessage
          ? null
          : (autoFetchMessage ?? current.autoFetchMessage),
      errorMessage: clearError ? null : (errorMessage ?? current.errorMessage),
      progress: progress ?? current.progress,
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

  /// Codes VTOP rejected in this challenge; fetching skips them.
  final _rejectedCodes = <String>{};
  int _autoResends = 0;

  /// The OTP comes by forwarded mail (pushed) rather than Gmail polling.
  bool _useMailForwarding = false;
  String _logContext = 'otp.flow';

  // Logs under `client.otp`, prefixed with this challenge's id.
  void _info(String m) => Log.info('client.otp', '$_logContext $m');
  void _warn(String m) => Log.warn('client.otp', '$_logContext $m');
  void _error(String m) => Log.error('client.otp', '$_logContext $m');

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
    String message = defaultOtpMessage,
    DateTime? otpRequiredAt,
    String? logContext,
  }) async {
    _client = client;
    _otpRequiredAt = otpRequiredAt?.toUtc() ?? DateTime.now().toUtc();
    _logContext = logContext ?? 'otp.flow#${++_challengeCounter}';
    final autoFetch = _emailAutoFetchAvailable = await canAutoFetchFromEmail();
    final progress = await _progressFor(
      canAutoFetchFromEmail: autoFetch,
      requestedAt: _otpRequiredAt!,
    );
    final activity = autoFetch
        ? OtpChallengeActivity.fetchingEmail
        : OtpChallengeActivity.waiting;
    final autoFetchMessage = autoFetch ? _tryingEmail : null;
    if (_completer case final completer?
        when state.isActive && !completer.isCompleted) {
      _info('challenge already active; refreshing prompt state');
      state = state.update(
        presentation: .expanded,
        message: message,
        activity: activity,
        canRetryEmailAutoFetch: false,
        autoFetchMessage: autoFetchMessage,
        clearAutoFetchMessage: !autoFetch,
        progress: progress,
        clearError: true,
      );
      return completer.future;
    }

    final completer = _completer = Completer<void>();
    _rejectedCodes.clear();
    _autoResends = 0;
    _info('challenge started (emailAutofetch=$autoFetch)');
    state = VtopOtpChallengeState.active(
      presentation: autoFetch ? .minimized : .expanded,
      activity: activity,
      remainingSeconds: _otpChallengeTimeout.inSeconds,
      message: message,
      autoFetchMessage: autoFetchMessage,
      progress: progress,
    );
    _startTicker();
    if (autoFetch) _startAutoFetch();
    return completer.future;
  }

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

    _info('retrying email autofetch');
    state = state.update(
      activity: OtpChallengeActivity.fetchingEmail,
      canRetryEmailAutoFetch: false,
      message:
          'Trying to get OTP from email again. You can still enter it manually.',
      autoFetchMessage: _tryingEmail,
      clearError: true,
    );
    _startAutoFetch();
  }

  void minimize() {
    if (!state.isActive) return;
    state = state.update(presentation: .minimized);
  }

  void reopen() {
    if (!state.isActive) return;
    state = state.update(presentation: .expanded);
  }

  void cancel() {
    _warn('challenge cancelled');
    _finishWithError(
      VtopError.authenticationFailed('OTP verification cancelled'),
    );
  }

  Future<void> submitOtp(String otp) async {
    if (!state.isActive || _client == null) return;
    final sanitizedOtp = otp.replaceAll(RegExp(r'\D'), '');
    if (sanitizedOtp.length != 6) {
      _warn('rejected OTP submit because the code was not 6 digits');
      state = state.update(errorMessage: 'Please enter a valid 6-digit OTP.');
      return;
    }

    _info('submitting OTP code');
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
      _info('OTP accepted');
      _finishSuccess();
    } catch (error) {
      final message = _authMessage(error);
      if (_rejectionOf(error, message) case final rejection?) {
        _warn('OTP rejected (${rejection.name})');
        _onRejected(sanitizedOtp, rejection, message);
        return;
      }
      _error('OTP submission failed: $error');
      _finishWithError(error);
    }
  }

  /// What happens after VTOP rejects [code]: wait for a newer code, send a
  /// new one when it expired, or ask by hand with Try again.
  void _onRejected(String code, _Rejection rejection, String message) {
    _rejectedCodes.add(code);
    if (rejection == _Rejection.expired) {
      _ticker?.cancel();
      state = state.update(
        activity: .waiting,
        presentation: .expanded,
        remainingSeconds: 0,
        message: _expiredMessage,
        errorMessage: message,
        clearAutoFetchMessage: true,
      );
      if (_canAutoResend) {
        _autoResends++;
        unawaited(
          resendOtp(
            message:
                'That OTP had expired, so a new one was sent. Waiting for it…',
          ),
        );
      }
      return;
    }
    if (_emailAutoFetchAvailable &&
        _rejectedCodes.length < _maxRejectedCodes &&
        state.remainingSeconds > 0) {
      state = state.update(
        activity: .fetchingEmail,
        presentation: .expanded,
        message: keepTryingMessage,
        autoFetchMessage: 'Waiting for a newer OTP…',
        errorMessage: message,
      );
      _startAutoFetch();
      return;
    }
    state = state.update(
      activity: .waiting,
      presentation: .expanded,
      canRetryEmailAutoFetch:
          _emailAutoFetchAvailable && state.remainingSeconds > 0,
      errorMessage: message,
      clearAutoFetchMessage: true,
    );
  }

  Future<void> resendOtp({
    String message = 'A new OTP has been sent to your registered email.',
  }) async {
    if (!state.isActive || _client == null) return;
    if (state.remainingSeconds > 0 || state.isSubmitting || state.isResending) {
      return;
    }

    state = state.update(
      activity: .resending,
      canRetryEmailAutoFetch: false,
      clearAutoFetchMessage: true,
      clearError: true,
    );
    _info('requesting OTP resend');
    try {
      final requestedAt = DateTime.now().toUtc();
      await ref.read(vtopAuthenticatorProvider).resendOtp(_client!);
      _otpRequiredAt = requestedAt;
      _info('resend completed');
      state = state.update(
        activity: _emailAutoFetchAvailable
            ? OtpChallengeActivity.fetchingEmail
            : OtpChallengeActivity.waiting,
        remainingSeconds: _otpChallengeTimeout.inSeconds,
        message: message,
        autoFetchMessage: _emailAutoFetchAvailable ? _tryingEmail : null,
        progress: state.progress?.restarted(requestedAt),
        clearError: true,
      );
      _startTicker();
      if (state.isAutoFetchingEmail) _startAutoFetch();
    } catch (error) {
      _error('resend failed: $error');
      state = state.update(
        activity: .waiting,
        errorMessage: _authMessage(error),
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
        if (_canAutoResend && !state.isSubmitting && !state.isResending) {
          _autoResends++;
          state = state.update(
            activity: .waiting,
            remainingSeconds: 0,
            clearAutoFetchMessage: true,
          );
          unawaited(
            resendOtp(
              message:
                  'The OTP expired, so a new one was sent. Waiting for it…',
            ),
          );
          return;
        }
        final autoFetchPending = state.isAutoFetchingEmail;
        state = state.update(
          activity: .waiting,
          remainingSeconds: 0,
          message: _expiredMessage,
          errorMessage: autoFetchPending
              ? null
              : 'OTP expired. Please resend OTP.',
          clearAutoFetchMessage: true,
        );
        return;
      }
      state = state.update(remainingSeconds: next);
    });
  }

  bool get _canAutoResend =>
      _emailAutoFetchAvailable && _autoResends < _maxAutoResends;

  void _startAutoFetch() =>
      unawaited(_runEmailAutoFetch(runId: ++_autoFetchRunId));

  void _finishSuccess() {
    _info('challenge completed');
    _reset()?.complete();
  }

  void _finishWithError(Object error) {
    _warn('challenge finished with error: $error');
    _reset()?.completeError(error);
  }

  /// Back to idle; returns the challenge's completer if it is still open.
  Completer<void>? _reset() {
    final completer = _completer;
    _autoFetchRunId++;
    _ticker?.cancel();
    _ticker = null;
    _client = null;
    _otpRequiredAt = null;
    _emailAutoFetchAvailable = false;
    _useMailForwarding = false;
    _rejectedCodes.clear();
    _autoResends = 0;
    _completer = null;
    state = const VtopOtpChallengeState.idle();
    return completer != null && !completer.isCompleted ? completer : null;
  }

  /// Forwarded mail comes first (pushed, so it's fastest); Gmail (own key
  /// or shared access) is the second choice.
  Future<bool> canAutoFetchFromEmail() async {
    try {
      _useMailForwarding =
          !await isOnDeviceMode() &&
          await ref.read(mailForwardingServiceProvider).isReady();
      return _useMailForwarding || await _gmailReady();
    } catch (error, stackTrace) {
      _error('failed to evaluate email OTP autofetch availability: $error');
      Zone.current.handleUncaughtError(error, stackTrace);
      return false;
    }
  }

  /// What the prompt shows for the source [canAutoFetchFromEmail] chose.
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

  Future<bool> _gmailReady() =>
      ref.read(googleEmailOtpAuthServiceProvider).isReady();

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
    _info('email autofetch started from ${startedAt.toIso8601String()}');
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
        if (otp != null && otp.isNotEmpty && !_rejectedCodes.contains(otp)) {
          _info('email autofetch found an OTP on attempt $attempt');
          return otp;
        }
      } catch (error, stackTrace) {
        _error('email autofetch failed on attempt $attempt: $error');
        if (stop != null) {
          _updateProgress(
            (p) => p.checked(problem: 'Could not read Gmail too.'),
          );
          return null;
        }
        Zone.current.handleUncaughtError(error, stackTrace);
        if (!_shouldContinueAutoFetch(runId)) return null;
        state = state.update(
          activity: .waiting,
          canRetryEmailAutoFetch: true,
          presentation: .expanded,
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

  /// Waits for the forwarded OTP mail: over the edge's event stream or FCM, and
  /// pulled every 2 s while no stream is open, in case a push is slow or
  /// held back, and on resume.
  /// With Gmail also connected, reads Gmail too if nothing came in 30 s.
  Future<void> _runMailForwardingAutoFetch({required int runId}) async {
    final service = ref.read(mailForwardingServiceProvider);
    final startedAt = _otpRequiredAt!;
    _info('waiting for a forwarded OTP from ${startedAt.toIso8601String()}');
    state = state.update(autoFetchMessage: 'Waiting for the OTP email...');

    final found = Completer<String?>();
    void take(MailOtpPayload payload) {
      if (!found.isCompleted &&
          payload.isOtpFor(startedAt) &&
          !_rejectedCodes.contains(payload.code)) {
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
        _warn('forwarded OTP pull failed: $error');
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
    // The mail arrives at once over the event stream, which is otherwise
    // closed for an install that is not linked.
    final releaseStream = EdgeEventChannel.instance.hold();
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
      _info('no forwarded OTP yet; reading Gmail too');
      _updateProgress((p) => p.withGmailFallback());
      final otp = await _fetchFromGmail(
        runId: runId,
        stop: () => found.isCompleted,
      );
      if (otp != null && !found.isCompleted) found.complete(otp);
    });

    final code = await found.future;
    unawaited(subscription.cancel());
    releaseStream();
    lifecycle.dispose();
    watch.cancel();
    gmailFallback.cancel();
    poll.cancel();
    if (code == null || !_shouldContinueAutoFetch(runId)) return;
    _info('forwarded OTP arrived');
    await submitOtp(code);
  }

  bool _shouldContinueAutoFetch(int runId) =>
      runId == _autoFetchRunId &&
      state.isAutoFetchingEmail &&
      state.remainingSeconds > 0 &&
      _client != null;
}

/// VTOP turned the code down: [_Rejection.expired] when it says so,
/// otherwise [_Rejection.invalid] whatever the wording. Other errors end
/// the challenge.
_Rejection? _rejectionOf(Object error, String message) => switch (error) {
  VtopError_AuthenticationFailed() =>
    message.toLowerCase().contains('expire') ? .expired : .invalid,
  _ => null,
};

String _authMessage(Object error) => switch (error) {
  VtopError_AuthenticationFailed(:final field0) when field0.trim().isNotEmpty =>
    field0.trim(),
  _ => 'Authentication failed',
};
