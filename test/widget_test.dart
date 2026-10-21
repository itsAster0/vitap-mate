import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:forui/forui.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/email_otp/google_oauth_loopback.dart';
import 'package:vitapmate/features/more/presentation/pages/chrome_extension_page.dart';
import 'package:vitapmate/features/more/presentation/pages/more_page.dart';
import 'package:vitapmate/features/more/presentation/pages/gpa_calculator_page.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/settings/presentation/pages/gmail_otp_setup_page.dart';
import 'package:vitapmate/features/settings/presentation/pages/gmail_oauth_guide_page.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

GoogleEmailOtpAuthService _testOAuthService() {
  final httpClient = MockClient((_) async => http.Response('{}', 500));
  return GoogleEmailOtpAuthService(
    appAuth: const FlutterAppAuth(),
    storage: const FlutterSecureStorage(),
    httpClient: httpClient,
    loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
  );
}

Widget _otpScope(GoogleEmailOtpAuthService service, Widget page) =>
    ProviderScope(
      overrides: [googleEmailOtpAuthServiceProvider.overrideWithValue(service)],
      child: MaterialApp(
        builder: (context, child) => FTheme(
          data: FTheme.neutral.light.touch,
          child: FToaster(child: child!),
        ),
        home: Scaffold(body: page),
      ),
    );

final _emptyGradeHistory = GradeHistoryData(
  student: const GradeHistoryStudentInfo(
    regNo: '',
    name: '',
    programmeBranch: '',
    programmeMode: '',
    studySystem: '',
    gender: '',
    yearJoined: '',
    eduStatus: '',
    school: '',
    campus: '',
  ),
  records: const [],
  cgpa: const GradeHistoryCgpa(
    creditsRegistered: '',
    creditsEarned: '',
    cgpa: '',
    sGrades: '',
    aGrades: '',
    bGrades: '',
    cGrades: '',
    dGrades: '',
    eGrades: '',
    fGrades: '',
    nGrades: '',
  ),
  updateTime: BigInt.zero,
);

final _emptyTimetable = TimetableData(
  slots: const [],
  courses: const [],
  semesterId: '',
  updateTime: BigInt.zero,
);

void main() {
  testWidgets('Email OTP offers forwarding first, own key second', (
    tester,
  ) async {
    FlutterSecureStorage.setMockInitialValues({});
    await tester.pumpWidget(
      _otpScope(
        _testOAuthService(),
        const GmailOtpSetupPage(
          androidByokOverride: true,
          sharedOAuthOverride: true,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('Not set up'), findsOneWidget);
    expect(find.text('OPTION 1 · RECOMMENDED'), findsOneWidget);
    expect(find.text('Email forwarding'), findsOneWidget);
    expect(find.text('Start the guide'), findsOneWidget);
    expect(find.text('OPTION 2 · MOST CONTROL'), findsOneWidget);
    expect(find.text('Your own Google key'), findsOneWidget);
    expect(find.text('Set up forwarding'), findsOneWidget);
    expect(find.text('Shared Google access'), findsOneWidget);
    expect(find.text('Last resort · may stop working'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('Email forwarding')).dy,
      lessThan(tester.getTopLeft(find.text('Your own Google key')).dy),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('Google key guide walks through steps one at a time', (
    tester,
  ) async {
    FlutterSecureStorage.setMockInitialValues({});
    await tester.pumpWidget(
      _otpScope(
        _testOAuthService(),
        const GmailOAuthGuidePage(androidOverride: true),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Step 1 of 7'), findsOneWidget);
    expect(find.text('Create a Google Cloud project'), findsOneWidget);
    expect(find.text('Add your college email as a test user'), findsOneWidget);
    expect(find.text('Publish the app'), findsOneWidget);
    expect(find.text('Stops weekly re-login'), findsOneWidget);
    expect(find.textContaining('not your college account'), findsOneWidget);
    // Only the open step shows its details.
    expect(find.text('Mark done'), findsOneWidget);
    expect(find.textContaining('Pick Desktop app'), findsNothing);
    expect(find.text('Import JSON'), findsOneWidget);
    expect(find.text('Something went wrong?'), findsOneWidget);

    await tester.ensureVisible(find.text('Mark done'));
    await tester.tap(find.text('Mark done'));
    await tester.pumpAndSettle();
    expect(find.text('Step 2 of 7'), findsOneWidget);
    expect(find.text('Tap Enable.'), findsOneWidget);

    await tester.ensureVisible(
      find.text('Create a Desktop app client, download the JSON'),
    );
    await tester.tap(
      find.text('Create a Desktop app client, download the JSON'),
    );
    await tester.pumpAndSettle();
    expect(
      find.textContaining('Pick Desktop app, not Android or Web'),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('Email OTP hides shared access when no build key exists', (
    tester,
  ) async {
    FlutterSecureStorage.setMockInitialValues({});
    await tester.pumpWidget(
      _otpScope(
        _testOAuthService(),
        const GmailOtpSetupPage(
          androidByokOverride: true,
          sharedOAuthOverride: false,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Your own Google key'), findsOneWidget);
    expect(find.text('Shared Google access'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Email OTP refreshes the saved session when Android resumes', (
    tester,
  ) async {
    FlutterSecureStorage.setMockInitialValues({});
    const storage = FlutterSecureStorage();
    await tester.pumpWidget(
      _otpScope(
        _testOAuthService(),
        const GmailOtpSetupPage(
          androidByokOverride: true,
          sharedOAuthOverride: false,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('Connected · '), findsNothing);

    const session = EmailOtpOAuthSession(
      email: 'student.23abc4567@vitapstudent.ac.in',
      accessToken: 'access',
      refreshToken: 'refresh',
      scopes: ['https://www.googleapis.com/auth/gmail.modify'],
      accessTokenExpiryEpochMs: 4102444800000,
      authSource: EmailOtpAuthSource.personalByok,
      oauthClientId: 'personal.apps.googleusercontent.com',
    );
    await storage.write(
      key: 'email_otp_oauth_session_v1',
      value: jsonEncode(session.toJson()),
    );

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump(const Duration(milliseconds: 2100));

    expect(
      find.text('Connected · student.23abc4567@vitapstudent.ac.in'),
      findsOneWidget,
    );
    expect(find.text('Test Gmail'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Chrome extension guide explains setup with access keys', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: ChromeExtensionPage()),
        ),
      ),
    );

    expect(find.text('Download extension.zip'), findsOneWidget);
    expect(find.text('Paste an access key'), findsOneWidget);
    expect(find.text('Open Chrome extensions'), findsOneWidget);
    expect(find.textContaining('FCM'), findsNothing);
    expect(find.textContaining('FMC'), findsNothing);
  });

  testWidgets('More page exposes the GPA and CGPA calculator', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: MorePage()),
        ),
      ),
    );

    expect(find.text('GPA Planner'), findsOneWidget);
    expect(find.text('Chrome Extension'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('GPA calculator builds its semester view', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          gradeHistoryProvider.overrideWithBuild(
            (ref, notifier) async => _emptyGradeHistory,
          ),
          timetableProvider.overrideWithBuild(
            (ref, notifier) async => _emptyTimetable,
          ),
        ],
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: GpaCalculatorPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('SEMESTER GPA'), findsOneWidget);
    expect(find.text('PROJECTED CGPA'), findsOneWidget);
    expect(find.text('Current semester'), findsOneWidget);
    expect(find.text('Add course'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
