import 'dart:async';

import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/widgets/onboarding_page.dart';
import 'package:vitapmate/core/widgets/shell_layout.dart';
import 'package:vitapmate/features/attendance/presentation/pages/attendance_page.dart';
import 'package:vitapmate/features/more/presentation/pages/exam_schedule_page.dart';
import 'package:vitapmate/features/more/presentation/pages/marks_page.dart';
import 'package:vitapmate/features/more/presentation/pages/more_page.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview.dart';
import 'package:vitapmate/features/settings/presentation/pages/settings_page.dart';
import 'package:vitapmate/features/settings/presentation/pages/notification_management_page.dart';
import 'package:vitapmate/features/settings/presentation/pages/user_management.dart';
import 'package:vitapmate/features/social/presentation/pages/message_chat_page.dart';
import 'package:vitapmate/features/social/presentation/pages/social_page.dart';
import 'package:vitapmate/features/timetable/presentation/pages/timetable_page.dart';

final rootNavigatorKey = GlobalKey<NavigatorState>();

final routerProvider = Provider((ref) {
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
            child: VtopWebview(),
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
                    path: '/vtopUserManagement',
                    name: Paths.vtopUserManagement,

                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: UserManagementPage(),
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
                path: '/social',
                name: Paths.social,
                builder: (context, state) => SocialPage(),
                routes: [
                  GoRoute(
                    path: 'messagechat',
                    name: Paths.messageChat,

                    pageBuilder: (context, state) {
                      return NoTransitionPage<void>(
                        key: state.pageKey,
                        child: MessageChatPage(),
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
});

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
