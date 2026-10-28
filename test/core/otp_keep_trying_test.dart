import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/vtop/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/vtop/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/vtop/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/vtop/backend/vtop_authenticator.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

class _Client implements VtopClient {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

/// VTOP: accepts only [good]; [reply] can override what a code gets.
class _Vtop implements VtopAuthenticator {
  _Vtop(this.good);

  String good;
  final submitted = <String>[];
  var resends = 0;
  final reply = <String, String>{};

  @override
  Future<void> submitOtp(VtopClient client, String otp) async {
    submitted.add(otp);
    if (otp == good) return;
    throw VtopError.authenticationFailed(
      reply[otp] ?? 'Invalid OTP. Please try again.',
    );
  }

  @override
  Future<void> resendOtp(VtopClient client) async => resends++;

  @override
  Future<void> login(VtopClient client) async {}
}

/// The forwarded inbox: what a pull returns right now.
class _Inbox implements MailForwardingService {
  MailOtpPayload? latest;

  void deliver(String code) => latest = MailOtpPayload(
    kind: 'otp',
    sentAt: DateTime.now().toUtc(),
    code: code,
  );

  @override
  Future<bool> isReady() async => true;

  @override
  Future<MailForwarding?> load() async => const MailForwarding(
    id: 'm',
    address: 'vtop-otp-a@aster0.dev',
    secret: 's',
    privateKey: 'k',
  );

  @override
  Future<MailOtpPayload?> pullInbox() async => latest;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _NoGmail implements GoogleEmailOtpAuthService {
  @override
  Future<bool> isReady() async => false;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late _Vtop vtop;
  late _Inbox inbox;

  /// Runs a challenge in fake time; [script] drives it.
  void challenge(
    void Function(
      FakeAsync time,
      VtopOtpChallenge notifier,
      ProviderContainer container,
      List<Object> outcome,
    )
    script,
  ) {
    fakeAsync((time) {
      SharedPreferencesAsyncPlatform.instance =
          InMemorySharedPreferencesAsync.empty();
      final container = ProviderContainer(
        overrides: [
          vtopAuthenticatorProvider.overrideWithValue(vtop),
          mailForwardingServiceProvider.overrideWithValue(inbox),
          googleEmailOtpAuthServiceProvider.overrideWithValue(_NoGmail()),
        ],
      );
      final notifier = container.read(vtopOtpChallengeProvider.notifier);
      final outcome = <Object>[];
      notifier
          .requestOtp(client: _Client())
          .then(
            (_) => outcome.add('ok'),
            onError: (Object e) => outcome.add(e),
          );
      time.flushMicrotasks();
      script(time, notifier, container, outcome);
      notifier.cancel();
      time.flushMicrotasks();
      container.dispose();
    });
  }

  setUp(() {
    vtop = _Vtop('222222');
    inbox = _Inbox();
  });

  test('an old OTP is rejected, then the next one is used', () {
    challenge((time, notifier, container, outcome) {
      inbox.deliver('111111');
      time.elapse(const Duration(seconds: 3));
      expect(vtop.submitted, ['111111']);
      final state = container.read(vtopOtpChallengeProvider);
      expect(state.isAutoFetchingEmail, isTrue);
      expect(state.isMinimized, isFalse);
      expect(state.message, keepTryingMessage);

      // The rejected code is still the latest for a while: not sent again.
      time.elapse(const Duration(seconds: 10));
      expect(vtop.submitted, ['111111']);

      inbox.deliver('222222');
      time.elapse(const Duration(seconds: 3));
      expect(vtop.submitted, ['111111', '222222']);
      expect(outcome, ['ok']);
    });
  });

  test('a rejection worded differently still keeps trying', () {
    vtop.reply['111111'] = 'Wrong code entered';
    challenge((time, notifier, container, outcome) {
      inbox.deliver('111111');
      time.elapse(const Duration(seconds: 3));
      expect(outcome, isEmpty);
      expect(
        container.read(vtopOtpChallengeProvider).isAutoFetchingEmail,
        isTrue,
      );
    });
  });

  test('an expired OTP gets a new one sent, and that is used', () {
    vtop.reply['111111'] = 'OTP has expired. Please resend.';
    challenge((time, notifier, container, outcome) {
      inbox.deliver('111111');
      time.elapse(const Duration(seconds: 3));
      expect(vtop.resends, 1);
      inbox.deliver('222222');
      time.elapse(const Duration(seconds: 3));
      expect(vtop.submitted, ['111111', '222222']);
      expect(outcome, ['ok']);
    });
  });

  test('when the timer runs out a new OTP is sent, at most twice', () {
    challenge((time, notifier, container, outcome) {
      time.elapse(const Duration(minutes: 3, seconds: 2));
      expect(vtop.resends, 1);
      expect(
        container.read(vtopOtpChallengeProvider).isAutoFetchingEmail,
        isTrue,
      );
      time.elapse(const Duration(minutes: 7));
      expect(vtop.resends, 2);
      final state = container.read(vtopOtpChallengeProvider);
      expect(state.isActive, isTrue);
      expect(state.isAutoFetchingEmail, isFalse);
      expect(state.remainingSeconds, 0);
    });
  });

  test('after three rejected codes it stops and offers Try again', () {
    vtop.good = '999999';
    challenge((time, notifier, container, outcome) {
      for (final code in ['111111', '222222', '333333']) {
        inbox.deliver(code);
        time.elapse(const Duration(seconds: 3));
      }
      expect(vtop.submitted, ['111111', '222222', '333333']);
      final state = container.read(vtopOtpChallengeProvider);
      expect(state.isAutoFetchingEmail, isFalse);
      expect(state.canRetryEmailAutoFetch, isTrue);

      notifier.retryEmailAutoFetch();
      inbox.deliver('999999');
      time.elapse(const Duration(seconds: 3));
      expect(outcome, ['ok']);
    });
  });

  test('a typed wrong OTP keeps waiting for the forwarded one', () {
    challenge((time, notifier, container, outcome) {
      notifier.submitOtp('123456');
      time.flushMicrotasks();
      expect(
        container.read(vtopOtpChallengeProvider).isAutoFetchingEmail,
        isTrue,
      );
      inbox.deliver('222222');
      time.elapse(const Duration(seconds: 3));
      expect(outcome, ['ok']);
    });
  });
}
