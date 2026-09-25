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
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/features/more/presentation/pages/chrome_extension_page.dart';
import 'package:vitapmate/features/more/presentation/pages/more_page.dart';
import 'package:vitapmate/features/more/presentation/pages/gpa_calculator_page.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/settings/presentation/pages/gmail_otp_setup_page.dart';
import 'package:vitapmate/features/settings/presentation/pages/gmail_oauth_guide_page.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

const _sharedFallbackWarningForTest =
    'Fallback only. Shared Google access may be unavailable because of OAuth user limits, rate limits, verification status, or VIT Workspace policy. Try the personal BYOK method first.';

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
  testWidgets('Gmail setup prefers BYOK and clearly labels shared fallback', (
    tester,
  ) async {
    FlutterSecureStorage.setMockInitialValues({});
    final httpClient = MockClient((_) async => http.Response('{}', 500));
    final service = GoogleEmailOtpAuthService(
      appAuth: const FlutterAppAuth(),
      storage: const FlutterSecureStorage(),
      httpClient: httpClient,
      loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          googleEmailOtpAuthServiceProvider.overrideWithValue(service),
        ],
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(
            body: GmailOtpSetupPage(
              androidByokOverride: true,
              sharedOAuthOverride: true,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Use your own Google access'), findsOneWidget);
    expect(find.text('Preferred'), findsOneWidget);
    expect(find.text('Import OAuth JSON'), findsOneWidget);
    expect(find.text('How to get OAuth credentials'), findsOneWidget);
    expect(find.text('Create or select a Google Cloud project'), findsNothing);
    expect(find.text('Use shared app access'), findsOneWidget);
    expect(find.text('Fallback'), findsOneWidget);
    expect(find.text(_sharedFallbackWarningForTest), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Gmail OAuth guide explains Testing and shared JSON safety', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        builder: (context, child) => FTheme(
          data: FTheme.neutral.light.touch,
          child: FToaster(child: child!),
        ),
        home: const Scaffold(body: GmailOAuthGuidePage()),
      ),
    );

    expect(find.text('Get your Google OAuth file'), findsOneWidget);
    expect(
      find.text('Create or select a Google Cloud project'),
      findsOneWidget,
    );
    expect(find.text('Set the audience and test users'), findsOneWidget);
    expect(find.text('Open Google Cloud'), findsNWidgets(5));
    expect(find.textContaining('Keep that project selected'), findsOneWidget);
    expect(
      find.textContaining('Wait until the API overview page appears'),
      findsOneWidget,
    );
    expect(
      find.textContaining('This is the name Google shows during consent'),
      findsOneWidget,
    );
    expect(find.textContaining('Choose External—not Internal'), findsOneWidget);
    expect(find.textContaining('@vitapstudent.ac.in'), findsOneWidget);
    expect(
      find.textContaining('do not choose Android or Web application'),
      findsOneWidget,
    );
    expect(find.text('Return to Vitap Mate'), findsOneWidget);
    expect(find.textContaining('loopback OAuth documentation'), findsNothing);
    expect(find.text('Using a trusted friend’s JSON'), findsOneWidget);
    expect(
      find.textContaining('Never share your access token'),
      findsOneWidget,
    );
    expect(
      find.textContaining('Google hasn’t verified this app'),
      findsOneWidget,
    );
    expect(find.textContaining('gmail.modify'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Gmail setup hides shared access when no build key exists', (
    tester,
  ) async {
    FlutterSecureStorage.setMockInitialValues({});
    final httpClient = MockClient((_) async => http.Response('{}', 500));
    final service = GoogleEmailOtpAuthService(
      appAuth: const FlutterAppAuth(),
      storage: const FlutterSecureStorage(),
      httpClient: httpClient,
      loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          googleEmailOtpAuthServiceProvider.overrideWithValue(service),
        ],
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(
            body: GmailOtpSetupPage(
              androidByokOverride: true,
              sharedOAuthOverride: false,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Use your own Google access'), findsOneWidget);
    expect(find.text('Use shared app access'), findsNothing);
    expect(find.text('Use shared fallback'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Gmail setup refreshes the saved session when Android resumes', (
    tester,
  ) async {
    FlutterSecureStorage.setMockInitialValues({});
    const storage = FlutterSecureStorage();
    final httpClient = MockClient((_) async => http.Response('{}', 500));
    final service = GoogleEmailOtpAuthService(
      appAuth: const FlutterAppAuth(),
      storage: storage,
      httpClient: httpClient,
      loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          googleEmailOtpAuthServiceProvider.overrideWithValue(service),
        ],
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(
            body: GmailOtpSetupPage(
              androidByokOverride: true,
              sharedOAuthOverride: false,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Personal BYOK connected'), findsNothing);

    const session = EmailOtpOAuthSession(
      email: 'student.24mic7076@vitapstudent.ac.in',
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

    expect(find.text('Personal BYOK connected'), findsOneWidget);
    expect(find.text('student.24mic7076@vitapstudent.ac.in'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Chrome extension guide explains setup and uses Token wording', (
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

    expect(find.text('Copy Token'), findsOneWidget);
    expect(find.text('Reset Token'), findsOneWidget);
    expect(find.text('Open GitHub release'), findsOneWidget);
    expect(find.text('Download extension.zip'), findsOneWidget);
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

  testWidgets('More page exposes Chrome Extension when Firebase is ready', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          fcmCookieBridgeAvailableProvider.overrideWith((ref) async => true),
        ],
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: MorePage()),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Chrome Extension'), findsOneWidget);
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
