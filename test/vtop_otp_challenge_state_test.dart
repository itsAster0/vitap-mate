import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/di/provider/vtop_otp_challenge_provider.dart';

void main() {
  group('VtopOtpChallengeState', () {
    test('idle state cannot also be active', () {
      const state = VtopOtpChallengeState.idle();

      expect(state.isActive, isFalse);
      expect(state.isSubmitting, isFalse);
      expect(() => state.update(message: 'invalid'), throwsStateError);
    });

    test('one activity replaces competing operation booleans', () {
      final waiting = VtopOtpChallengeState.active(
        activity: OtpChallengeActivity.waiting,
        presentation: OtpChallengePresentation.expanded,
        remainingSeconds: 30,
        message: 'Waiting',
      );
      final submitting = waiting.update(
        activity: OtpChallengeActivity.submitting,
      );

      expect(submitting.isSubmitting, isTrue);
      expect(submitting.isResending, isFalse);
      expect(submitting.isAutoFetchingEmail, isFalse);
    });

    test('rejects email retry during another operation', () {
      expect(
        () => VtopOtpChallengeState.active(
          activity: OtpChallengeActivity.submitting,
          presentation: OtpChallengePresentation.expanded,
          remainingSeconds: 30,
          message: 'Submitting',
          canRetryEmailAutoFetch: true,
        ),
        throwsStateError,
      );
    });
  });
}
