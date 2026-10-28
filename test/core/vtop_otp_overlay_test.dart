import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/app/theme_provider.dart';
import 'package:vitapmate/app/vtop_otp_overlay.dart';

class _Challenge extends VtopOtpChallenge {
  _Challenge(this.initial);

  final VtopOtpChallengeState initial;

  @override
  VtopOtpChallengeState build() => initial;
}

void main() {
  final requestedAt = DateTime.now().toUtc().subtract(
    const Duration(seconds: 12),
  );

  VtopOtpChallengeState prompt(
    OtpFetchProgress progress, {
    bool fetching = true,
  }) => VtopOtpChallengeState.active(
    activity: fetching
        ? OtpChallengeActivity.fetchingEmail
        : OtpChallengeActivity.waiting,
    presentation: OtpChallengePresentation.expanded,
    remainingSeconds: 150,
    message: 'OTP sent to your registered email.',
    progress: progress,
  );

  Future<void> show(WidgetTester tester, VtopOtpChallengeState state) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          vtopOtpChallengeProvider.overrideWith(() => _Challenge(state)),
        ],
        child: MaterialApp(
          builder: (_, child) => FTheme(data: appLightTheme, child: child!),
          home: const VtopOtpOverlay(child: SizedBox.expand()),
        ),
      ),
    );
    await tester.pump();
  }

  group('OtpFetchProgress', () {
    test('a check counts and clears an old problem', () {
      final progress = OtpFetchProgress(
        source: OtpSource.gmail,
        requestedAt: requestedAt,
      ).checked(problem: 'x').checked();
      expect(progress.checks, 2);
      expect(progress.problem, isNull);
      expect(progress.lastCheckAt, isNotNull);
    });

    test('a new request keeps the source and starts counting again', () {
      final later = requestedAt.add(const Duration(minutes: 3));
      final progress = OtpFetchProgress(
        source: OtpSource.forwarding,
        requestedAt: requestedAt,
        account: 'vtop-otp-abc@example.dev',
        gmailFallbackAvailable: true,
      ).checked().withGmailFallback().restarted(later);
      expect(progress.source, OtpSource.forwarding);
      expect(progress.account, 'vtop-otp-abc@example.dev');
      expect(progress.requestedAt, later);
      expect(progress.checks, 0);
      expect(progress.gmailFallback, isFalse);
      expect(progress.gmailFallbackAvailable, isTrue);
    });
  });

  testWidgets('forwarding shows the address, the wait and the Gmail fallback', (
    tester,
  ) async {
    await show(
      tester,
      prompt(
        OtpFetchProgress(
          source: OtpSource.forwarding,
          requestedAt: requestedAt,
          account: 'vtop-otp-abc@example.dev',
          gmailFallbackAvailable: true,
        ),
      ),
    );
    expect(find.text('Forwarded email'), findsOneWidget);
    expect(find.text('vtop-otp-abc@example.dev'), findsOneWidget);
    expect(
      find.text('Waiting for Gmail to forward the OTP email'),
      findsOneWidget,
    );
    expect(find.textContaining('No live connection'), findsOneWidget);
    expect(find.textContaining('Reads Gmail too in 0:'), findsOneWidget);
  });

  testWidgets('Gmail shows the account, checks and trash note', (tester) async {
    await show(
      tester,
      prompt(
        OtpFetchProgress(
          source: OtpSource.gmail,
          requestedAt: requestedAt,
          account: 'student@vitapstudent.ac.in',
          deleteAfterReading: true,
        ).checked().checked(),
      ),
    );
    expect(find.text('Gmail'), findsOneWidget);
    expect(find.text('student@vitapstudent.ac.in'), findsOneWidget);
    expect(find.textContaining('Checking your inbox · 2×'), findsOneWidget);
    expect(find.text('The OTP email goes to Trash once read'), findsOneWidget);
  });

  testWidgets('manual entry says how to turn on auto-fill', (tester) async {
    await show(
      tester,
      prompt(
        OtpFetchProgress(source: OtpSource.manual, requestedAt: requestedAt),
        fetching: false,
      ),
    );
    expect(find.text('Enter the code yourself'), findsOneWidget);
    expect(find.textContaining('set up Email OTP in Settings'), findsOneWidget);
  });

  testWidgets('a failed check is shown as a warning', (tester) async {
    await show(
      tester,
      prompt(
        OtpFetchProgress(
          source: OtpSource.forwarding,
          requestedAt: requestedAt,
        ).checked(
          problem: 'Could not reach the VITAP Mate server; still trying.',
        ),
      ),
    );
    expect(
      find.text('Could not reach the VITAP Mate server; still trying.'),
      findsOneWidget,
    );
  });
}
