import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/widgets/onboarding_page.dart';
import 'package:vitapmate/core/widgets/shell_layout.dart';
import 'package:vitapmate/features/attendance/presentation/pages/attendance_page.dart';
import 'package:vitapmate/features/more/presentation/pages/exam_schedule_page.dart';
import 'package:vitapmate/features/more/presentation/pages/grades_page.dart';
import 'package:vitapmate/features/more/presentation/pages/grade_history_page.dart';
import 'package:vitapmate/features/more/presentation/pages/marks_page.dart';
import 'package:vitapmate/features/more/presentation/pages/more_page.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview.dart';
import 'package:vitapmate/features/settings/presentation/pages/settings_page.dart';
import 'package:vitapmate/features/settings/presentation/pages/notification_management_page.dart';
import 'package:vitapmate/features/settings/presentation/pages/logs_page.dart';
import 'package:vitapmate/features/timetable/presentation/pages/timetable_page.dart';
import 'package:vitapmate/features/timetable/presentation/pages/calendar_sync_page.dart';

part 'router.g.dart';

final rootNavigatorKey = GlobalKey<NavigatorState>();

@Riverpod(keepAlive: true)
GoRouter router(Ref ref) {
  return GoRouter(
    redirect: (context, state) => redirect(context, ref, state),
    initialLocation: '/timetable',
    navigatorKey: rootNavigatorKey,
    routes: [
      GoRoute(
        path: '/onboarding',
        name: Paths.onbaording,
        builder: (context, state) => OnboardingPage(),
      ),
      GoRoute(
        path: '/vtopweb',
        name: Paths.vtopweb,

        pageBuilder: (context, state) {
          return NoTransitionPage<void>(
            key: state.pageKey,
            child: VtopWebview(initialMenuUrl: state.extra as String?),
          );
        },
      ),

      StatefulShellRoute.indexedStack(
        builder: (context, state, child) {
          return ShellLayout(child: child);
        },
        branches: [
          StatefulShellBranch(
            navigatorKey: GlobalKey<NavigatorState>(),
            routes: [
              GoRoute(
                path: '/timetable',
                name: Paths.timetable,
                builder: (context, state) => TimetablePage(),
                routes: [
                  GoRoute(
                    path: 'details',
                    builder: (context, state) => Placeholder(),
                  ),
                  GoRoute(
                    path: 'calendar-sync',
                    name: Paths.calendarSync,
                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: CalendarSyncPage(),
                      );
                    },
                  ),
                ],
              ),
            ],
          ),

          StatefulShellBranch(
            navigatorKey: GlobalKey<NavigatorState>(),
            routes: [
              GoRoute(
                path: '/attendance',
                name: Paths.attendance,

                builder: (context, state) => AttendancePage(),
              ),
            ],
          ),
          StatefulShellBranch(
            navigatorKey: GlobalKey<NavigatorState>(),
            routes: [
              GoRoute(
                path: '/more',
                name: Paths.more,
                builder: (context, state) => MorePage(),
                routes: [
                  GoRoute(
                    path: 'marks',
                    name: Paths.marks,

                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: MarksPage(),
                      );
                    },
                  ),
                  GoRoute(
                    path: 'grades',
                    name: Paths.grades,

                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: GradesPage(),
                      );
                    },
                  ),
                  GoRoute(
                    path: 'grade_history',
                    name: Paths.gradeHistory,
                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: GradeHistoryPage(),
                      );
                    },
                  ),
                  GoRoute(
                    path: 'exam_schedule',
                    name: Paths.examSchedule,

                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: ExamSchedulePage(),
                      );
                    },
                  ),
                ],
              ),
            ],
          ),
          StatefulShellBranch(
            navigatorKey: GlobalKey<NavigatorState>(),
            routes: [
              GoRoute(
                path: '/settings',
                name: Paths.settings,
                builder: (context, state) => SettingsPage(),
                routes: [
                  GoRoute(
                    path: 'notification-management',
                    name: Paths.notificationManagement,
                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: NotificationManagementPage(),
                      );
                    },
                  ),
                  GoRoute(
                    path: 'logs',
                    name: Paths.logs,
                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: LogsPage(),
                      );
                    },
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    ],
  );
}

FutureOr<String?> redirect(
  BuildContext context,
  Ref ref,
  GoRouterState state,
) async {
  String? k;
  try {
    var user = await ref.read(vtopUserProvider.future);
    if (user.username == null) {
      return '/onboarding';
    }
  } catch (e) {
    return '/onboarding';
  }
  return k;
}
