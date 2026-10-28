import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/timetable/widgets/timetable_view_toggle_button.dart';
import 'package:vitapmate/features/attendance/attendance_guide_page.dart';
import 'package:vitapmate/vtop/global_async_queue_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/app/router/paths.dart';
import 'package:vitapmate/platform/background/stale_refresh.dart';
import 'package:vitapmate/vtop/semesters.dart';
import 'package:vitapmate/features/docs/docs_provider.dart';
import 'package:vitapmate/features/server/widgets/server_format.dart';

class ShellLayout extends HookConsumerWidget {
  final Widget child;
  const ShellLayout({super.key, required this.child});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final sems = ref
        .watch(semesterIdProvider)
        .maybeWhen(data: (value) => value, orElse: () => null);
    final sem = ref
        .watch(vtopUserProvider)
        .maybeWhen(data: (value) => value, orElse: () => null);
    final newSemExist = useMemoized(() {
      final semid = sem?.semid;
      if (sems == null || semid == null) return false;
      int number(String id) =>
          int.tryParse(id.toLowerCase().replaceAll("ap", "")) ?? 0;
      final newest = sems.semesters.map((e) => number(e.id)).fold(0, math.max);
      return newest != 0 && number(semid) < newest;
    }, [sem?.semid, sems?.semesters.length]);
    // Refreshes stale pages after a quick fetch while the app is open.
    ref.watch(staleRefresherProvider);
    final runningTasks = ref.watch(
      globalAsyncQueueProvider.select((value) => value.running.keys.toList()),
    );
    final activeDocumentTitle = ref.watch(activeDocumentTitleProvider);

    // The URL does not change on push(), so read the deepest match from the
    // delegate instead; otherwise pushed pages keep the tab's header.
    final routerDelegate = GoRouter.of(context).routerDelegate;
    useListenable(routerDelegate);
    final configuration = routerDelegate.currentConfiguration;
    final location =
        configuration.lastOrNull?.matchedLocation ?? configuration.uri.path;
    final queueStatus = _queueStatusText(runningTasks);
    final isNestedRoute = location.split('/').length - 1 > 1;
    final headerSubtitle = isNestedRoute
        ? queueStatus
        : queueStatus ?? (newSemExist ? "New semester data available!" : null);
    final selected = useState(0);
    useEffect(() {
      if (_tabs.indexWhere((tab) => tab.prefixes.any(location.startsWith))
          case final index when index >= 0) {
        selected.value = index;
      }
      return null;
    }, [location]);

    final tabFade = useAnimationController(
      duration: const Duration(milliseconds: 200),
      initialValue: 1,
    );
    final tabOpacity = useMemoized(
      () => Tween<double>(
        begin: 0.3,
        end: 1,
      ).animate(CurvedAnimation(parent: tabFade, curve: Curves.easeOut)),
      [tabFade],
    );
    final previousTab = usePrevious(selected.value);
    final reduceMotion = MediaQuery.disableAnimationsOf(context);
    useEffect(() {
      if (previousTab != null &&
          previousTab != selected.value &&
          !reduceMotion) {
        tabFade.forward(from: 0);
      }
      return null;
    }, [selected.value]);

    return FScaffold(
      childPad: false,
      scaffoldStyle: FScaffoldStyleDelta.delta(
        footerDecoration: DecorationDelta.value(
          BoxDecoration(color: context.theme.colors.background),
        ),
      ),
      header: _buildHeader(
        context,
        _tabs[selected.value].route == Paths.docs &&
                location.startsWith('/docs/view')
            ? activeDocumentTitle ?? 'Document'
            : _tabs[selected.value].label,
        location,
        headerSubtitle,
      ),
      footer: SafeArea(
        top: false,
        right: false,
        left: false,
        bottom: true,
        child: FBottomNavigationBar(
          index: selected.value,
          style: FBottomNavigationBarStyleDelta.delta(
            decoration: DecorationDelta.value(
              BoxDecoration(color: context.theme.colors.background),
            ),
          ),
          onChange: (index) {
            if (index != selected.value) HapticFeedback.selectionClick();
            selected.value = index;
            GoRouter.of(context).goNamed(_tabs[index].route);
          },
          children: [
            for (final tab in _tabs)
              FBottomNavigationBarItem(
                icon: Icon(tab.icon),
                label: Text(tab.label),
              ),
          ],
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8.0),
        // The tabs share one navigator shell, so fade the newly shown tab in
        // rather than cross-fading two copies of it.
        child: FadeTransition(opacity: tabOpacity, child: child),
      ),
    );
  }
}

Widget _buildHeader(
  BuildContext context,
  String data,
  String path,
  String? subtitle,
) {
  if (path.split('/').length - 1 > 1) {
    data = switch (path.split('/')[2]) {
      'marks' => 'Marks',
      'grades' => 'Grades',
      'grade_history' => 'Grade History',
      'calendar-sync' => 'Calendar Sync',
      'notification-management' => 'Notifications',
      'debug' => switch (path.split('/').last) {
        'activity' => 'Activity',
        'diagnostics' => 'Diagnostics',
        'logs' => 'Logs',
        _ => 'Debug',
      },
      'exam_schedule' => 'Exam Schedule',
      'academic-calendar' => 'Academic Calendar',
      'biometric-history' => 'Biometric History',
      'profile' => 'Profile',
      'course-page' => path.endsWith('/course') ? 'Course' : 'Course Page',
      'outing' => path.endsWith('/pass') ? 'Outing Pass' : 'Outing',
      'server' => serverHeaderTitle(path),
      'gpa_calculator' => 'GPA Planner',
      'gmail-otp-setup' => 'Email OTP',
      'gmail-oauth-guide' => 'Your own Google key',
      'mail-forwarding' => 'Email forwarding',
      'how-it-works' => "How It's Calculated",
      _ => data,
    };

    return FHeader.nested(
      title: _HeaderTitle(
        title: data,
        subtitle: subtitle,
        titleStyle: _headerTitleStyle(context, subtitle),
      ),
      prefixes: [FHeaderAction.back(onPress: () => GoRouter.of(context).pop())],
    );
  }

  final hasTimetableAction = path.contains("timetable");
  return FHeader.nested(
    title: _HeaderTitle(
      title: data,
      subtitle: subtitle,
      titleStyle: _headerTitleStyle(context, subtitle),
    ),

    prefixes: [if (hasTimetableAction) const _AttendanceGuideButton()],

    suffixes: [
      if (hasTimetableAction) const TimetableViewToggleButton(),
      if (path.startsWith('/attendance')) const _AttendanceGuideButton(),
    ],
  );
}

/// Same title size on tabs and pushed pages, stepping down when a status
/// line shares the header.
TextStyle _headerTitleStyle(BuildContext context, String? subtitle) =>
    (subtitle != null
            ? context.theme.typography.body.sm
            : context.theme.typography.body.lg)
        .copyWith(fontWeight: FontWeight.w600);

class _HeaderTitle extends StatelessWidget {
  const _HeaderTitle({
    required this.title,
    required this.subtitle,
    required this.titleStyle,
  });

  final String title;
  final String? subtitle;
  final TextStyle titleStyle;

  @override
  Widget build(BuildContext context) {
    return Center(
      widthFactor: 1,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Text(
            title,
            style: titleStyle,
            textAlign: TextAlign.center,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          if (subtitle != null)
            Text(
              subtitle!,
              style: context.theme.typography.body.sm.copyWith(
                color: context.theme.colors.mutedForeground,
                fontWeight: FontWeight.w500,
              ),
              textAlign: TextAlign.center,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
        ],
      ),
    );
  }
}

String? _queueStatusText(List<String> runningTasks) {
  if (runningTasks.isEmpty) return null;
  final visibleTasks = runningTasks.map(_taskLabel).toSet();
  final extraCount = visibleTasks.length - 1;
  return extraCount <= 0
      ? visibleTasks.first
      : '${visibleTasks.first} + $extraCount more';
}

/// Queue task id prefixes and what the header says while they run.
const _taskLabels = [
  ('vtop_login', 'Logging in to VTOP...'),
  ('vtop_attendance', 'Fetching attendance...'),
  ('vtop_fullattendance', 'Fetching attendance details...'),
  ('vtop_timetable', 'Fetching timetable...'),
  ('vtop_fetchSchedule', 'Fetching exam schedule...'),
  ('vtop_marks', 'Fetching marks...'),
  ('vtop_grades', 'Fetching grades...'),
  ('vtop_grade_details', 'Fetching grade details...'),
  ('vtop_grade_history', 'Fetching grade history...'),
  ('vtop_semids', 'Fetching semesters...'),
  ('toStorage', 'Saving data...'),
  ('fromStorage', 'Loading data...'),
];

String _taskLabel(String id) {
  if (id == 'get_semids_storage') return 'Loading data...';
  for (final (prefix, label) in _taskLabels) {
    if (id.startsWith(prefix)) return label;
  }
  return 'Working...';
}

/// The bottom bar's tabs, in order, with the paths that select each.
const _tabs = [
  (
    label: 'Timetable',
    icon: FLucideIcons.calendarDays,
    route: Paths.timetable,
    prefixes: ['/timetable'],
  ),
  (
    label: 'Attendance',
    icon: FLucideIcons.userCheck,
    route: Paths.attendance,
    prefixes: ['/attendance'],
  ),
  (
    label: 'More',
    icon: FLucideIcons.libraryBig,
    route: Paths.more,
    prefixes: ['/more'],
  ),
  (
    label: 'Docs',
    icon: FLucideIcons.files,
    route: Paths.docs,
    prefixes: ['/docs', '/student-projects'],
  ),
  (
    label: 'Settings',
    icon: FLucideIcons.settings,
    route: Paths.settings,
    prefixes: ['/settings'],
  ),
];

/// Opens the page explaining how attendance numbers are worked out.
class _AttendanceGuideButton extends StatelessWidget {
  const _AttendanceGuideButton();

  @override
  Widget build(BuildContext context) => FButton.icon(
    semanticsLabel: 'How attendance is calculated',
    onPress: () => openAttendanceGuide(context),
    child: const Icon(FLucideIcons.info),
  );
}
