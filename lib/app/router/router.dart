import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/features/timetable/timetable_page.dart';
import 'package:vitapmate/features/timetable/calendar_sync_page.dart';
import 'package:vitapmate/features/exams/exam_schedule_page.dart';
import 'package:vitapmate/features/marks/marks_page.dart';
import 'package:vitapmate/features/grade_history/grade_history_page.dart';
import 'package:vitapmate/features/calendar/academic_calendar_page.dart';
import 'package:vitapmate/features/gpa/gpa_calculator_page.dart';
import 'package:vitapmate/features/grades/grades_page.dart';
import 'package:vitapmate/features/attendance/attendance_page.dart';
import 'package:vitapmate/features/biometric/biometric_history_page.dart';
import 'package:vitapmate/features/attendance/attendance_guide_page.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/app/router/paths.dart';
import 'package:vitapmate/app/router/slide_fade_page.dart';
import 'package:vitapmate/features/onboarding/onboarding_page.dart';
import 'package:vitapmate/app/shell_layout.dart';
import 'package:vitapmate/features/docs/document_viewer_page.dart';
import 'package:vitapmate/features/docs/docs_page.dart';
import 'package:vitapmate/features/docs/logic/doc_models.dart';
import 'package:vitapmate/features/more/chrome_extension_page.dart';
import 'package:vitapmate/features/more/more_page.dart';
import 'package:vitapmate/features/profile/profile_page.dart';
import 'package:vitapmate/features/outing/outing_page.dart';
import 'package:vitapmate/features/course_page/course_page_page.dart';
import 'package:vitapmate/features/course_page/course_view_page.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/vtop_browser/vtop_webview.dart';
import 'package:vitapmate/features/settings/settings_page.dart';
import 'package:vitapmate/features/server/server_address_page.dart';
import 'package:vitapmate/features/server/server_data_page.dart';
import 'package:vitapmate/features/server/server_apps_page.dart';
import 'package:vitapmate/features/server/server_phones_page.dart';
import 'package:vitapmate/features/developer/debug_page.dart';
import 'package:vitapmate/features/server/server_page.dart';
import 'package:vitapmate/features/account/mail_forwarding_page.dart';
import 'package:vitapmate/features/account/gmail_otp_setup_page.dart';
import 'package:vitapmate/features/account/gmail_oauth_guide_page.dart';
import 'package:vitapmate/features/settings/notification_management_page.dart';
import 'package:vitapmate/features/developer/logs_page.dart';
import 'package:vitapmate/app/widgets/server_feature_gate.dart';

final rootNavigatorKey = GlobalKey<NavigatorState>();

/// A route pushed with [SlideFadePage].
GoRoute _slide(
  String path,
  String name,
  Widget Function(GoRouterState state) page, {
  List<RouteBase> routes = const [],
}) => GoRoute(
  path: path,
  name: name,
  pageBuilder: (context, state) =>
      SlideFadePage<void>(key: state.pageKey, child: page(state)),
  routes: routes,
);

StatefulShellBranch _branch(RouteBase route) => StatefulShellBranch(
  navigatorKey: GlobalKey<NavigatorState>(),
  routes: [route],
);

final routerProvider = Provider<GoRouter>(
  (ref) => GoRouter(
    redirect: (context, state) async {
      try {
        final user = await ref.read(vtopUserProvider.future);
        return user.username == null ? '/onboarding' : null;
      } catch (_) {
        return '/onboarding';
      }
    },
    initialLocation: '/timetable',
    navigatorKey: rootNavigatorKey,
    routes: [
      GoRoute(
        path: '/onboarding',
        name: Paths.onboarding,
        builder: (context, state) => OnboardingPage(),
      ),
      _slide(
        '/vtopweb',
        Paths.vtopweb,
        (state) => VtopWebview(initialMenuUrl: state.extra as String?),
      ),
      StatefulShellRoute.indexedStack(
        builder: (context, state, child) => ShellLayout(child: child),
        branches: [
          _branch(
            _slide(
              '/timetable',
              Paths.timetable,
              (_) => TimetablePage(),
              routes: [
                GoRoute(
                  path: 'details',
                  builder: (context, state) => Placeholder(),
                ),
                _slide(
                  'how-it-works',
                  Paths.timetableAttendanceGuide,
                  (_) => const AttendanceGuidePage(showTimetable: true),
                ),
              ],
            ),
          ),
          _branch(
            _slide(
              '/attendance',
              Paths.attendance,
              (_) => AttendancePage(),
              routes: [
                _slide(
                  'how-it-works',
                  Paths.attendanceGuide,
                  (_) => const AttendanceGuidePage(),
                ),
              ],
            ),
          ),
          _branch(
            _slide(
              '/more',
              Paths.more,
              (_) => MorePage(),
              routes: [
                _slide('marks', Paths.marks, (_) => MarksPage()),
                _slide('grades', Paths.grades, (_) => GradesPage()),
                _slide(
                  'grade_history',
                  Paths.gradeHistory,
                  (_) => GradeHistoryPage(),
                ),
                _slide(
                  'exam_schedule',
                  Paths.examSchedule,
                  (_) => ExamSchedulePage(),
                ),
                _slide(
                  'academic-calendar',
                  Paths.academicCalendar,
                  (_) => const AcademicCalendarPage(),
                ),
                _slide(
                  'biometric-history',
                  Paths.biometricHistory,
                  (_) => const BiometricHistoryPage(),
                ),
                _slide(
                  'profile',
                  Paths.profile,
                  (state) => ProfilePage(
                    initialTab: switch (state.extra) {
                      final ProfileTab tab => tab,
                      _ => ProfileTab.profile,
                    },
                  ),
                ),
                _slide(
                  'outing',
                  Paths.outing,
                  (state) => OutingPage(
                    initialTab: switch (state.extra) {
                      final OutingTab tab => tab,
                      _ => OutingTab.general,
                    },
                  ),
                  routes: [
                    _slide(
                      'pass',
                      Paths.outingPass,
                      (state) =>
                          DocumentViewerPage(doc: state.extra as DocWindow),
                    ),
                  ],
                ),
                _slide(
                  'course-page',
                  Paths.coursePage,
                  (_) => const CoursePagePage(),
                  routes: [
                    _slide(
                      'course',
                      Paths.coursePageCourse,
                      (state) => CourseViewPage(
                        course: state.extra as CoursePageCourse,
                      ),
                    ),
                  ],
                ),
                GoRoute(
                  path: 'connected-apps',
                  name: Paths.connectedApps,
                  redirect: (_, _) => serverPagePath,
                ),
                _slide(
                  'gpa_calculator',
                  Paths.gpaCalculator,
                  (_) => const GpaCalculatorPage(),
                ),
              ],
            ),
          ),
          _branch(
            _slide(
              '/docs',
              Paths.docs,
              (_) => const DocsPage(),
              routes: [
                GoRoute(
                  path: 'view',
                  name: Paths.docView,
                  pageBuilder: (context, state) => CustomTransitionPage<void>(
                    key: state.pageKey,
                    child: DocumentViewerPage(doc: state.extra as DocWindow),
                    transitionsBuilder: (context, animation, secondary, child) {
                      final curved = CurvedAnimation(
                        parent: animation,
                        curve: Curves.easeOutCubic,
                      );
                      return FadeTransition(
                        opacity: curved,
                        child: ScaleTransition(
                          scale: Tween<double>(
                            begin: 0.97,
                            end: 1,
                          ).animate(curved),
                          child: child,
                        ),
                      );
                    },
                    transitionDuration: const Duration(milliseconds: 220),
                  ),
                ),
              ],
            ),
          ),
          _branch(
            _slide(
              '/settings',
              Paths.settings,
              (_) => SettingsPage(),
              routes: [
                _slide(
                  'server',
                  Paths.server,
                  (_) => const ServerPage(),
                  routes: [
                    _slide(
                      'apps',
                      Paths.serverApps,
                      (_) => const ServerAppsPage(),
                      routes: [
                        _slide(
                          'chrome-extension',
                          Paths.chromeExtension,
                          (_) => const ServerFeatureGate(
                            feature: 'The browser extension',
                            child: ChromeExtensionPage(),
                          ),
                        ),
                      ],
                    ),
                    _slide(
                      'phones',
                      Paths.serverPhones,
                      (_) => const ServerPhonesPage(),
                    ),
                    _slide(
                      'address',
                      Paths.serverAddress,
                      (_) => const ServerAddressPage(),
                    ),
                    _slide(
                      'data',
                      Paths.serverData,
                      (_) => const ServerDataPage(),
                    ),
                  ],
                ),
                _slide(
                  'debug',
                  Paths.debug,
                  (_) => const DebugPage(),
                  routes: [_slide('logs', Paths.debugLogs, (_) => LogsPage())],
                ),
                _slide(
                  'calendar-sync',
                  Paths.calendarSync,
                  (_) => CalendarSyncPage(),
                ),
                _slide(
                  'gmail-otp-setup',
                  Paths.gmailOtpSetup,
                  (_) => const GmailOtpSetupPage(),
                ),
                _slide(
                  'gmail-oauth-guide',
                  Paths.gmailOauthGuide,
                  (_) => const GmailOAuthGuidePage(),
                ),
                _slide(
                  'mail-forwarding',
                  Paths.mailForwarding,
                  (_) => const ServerFeatureGate(
                    feature: 'OTP mail forwarding',
                    child: MailForwardingPage(),
                  ),
                ),
                _slide(
                  'notification-management',
                  Paths.notificationManagement,
                  (_) => NotificationManagementPage(),
                ),
              ],
            ),
          ),
        ],
      ),
    ],
  ),
);
