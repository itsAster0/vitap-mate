import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
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
    // Quiet: only when nothing runs, after several failures in a row.
    final failing =
        ref.watch(backgroundFailuresProvider) >=
        ref.watch(tunableProvider(failureHintAfter));
    final headerSubtitle =
        queueStatus ??
        (failing ? 'Background refresh keeps failing' : null) ??
        (!isNestedRoute && newSemExist ? 'New semester data available!' : null);
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
        runningTasks,
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
  List<String> tasks,
) {
  if (path.split('/').length - 1 > 1) {
    data = switch (path.split('/')[2]) {
      'marks' => 'Marks',
      'grades' => 'Grades',
      'grade_history' => 'Grade History',
      'calendar-sync' => 'Calendar Sync',
      'notification-management' => 'Notifications',
      'connection' => 'Connection',
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
        tasks: tasks,
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
      tasks: tasks,
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

/// The page title, with the status line under it while VTOP work runs.
/// Tapping the status line lists every running task, with how long it has
/// run, until it or anywhere else is tapped. Tasks waiting on a sign-in are
/// dimmed.
class _HeaderTitle extends HookConsumerWidget {
  const _HeaderTitle({
    required this.title,
    required this.subtitle,
    required this.tasks,
    required this.titleStyle,
  });

  final String title;
  final String? subtitle;
  final List<String> tasks;
  final TextStyle titleStyle;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final list = useMemoized(OverlayPortalController.new);
    final link = useMemoized(LayerLink.new);
    final theme = context.theme;
    final muted = theme.typography.body.sm.copyWith(
      color: theme.colors.mutedForeground,
      fontWeight: .w500,
    );
    return Center(
      widthFactor: 1,
      child: GestureDetector(
        behavior: .opaque,
        onTap: list.toggle,
        child: CompositedTransformTarget(
          link: link,
          child: OverlayPortal(
            controller: list,
            overlayChildBuilder: (_) => Align(
              alignment: .topLeft,
              child: CompositedTransformFollower(
                link: link,
                targetAnchor: .bottomCenter,
                followerAnchor: .topCenter,
                offset: const Offset(0, 8),
                // The overlay sits outside the app's theme: style it here.
                child: TapRegion(
                  groupId: link,
                  onTapOutside: (_) => list.hide(),
                  child: _RunningTasks(
                    queue: ref.read(globalAsyncQueueProvider.notifier),
                    tasks: tasks,
                    theme: theme,
                  ),
                ),
              ),
            ),
            // Part of the list's tap region, so a tap here only toggles it.
            child: TapRegion(
              groupId: link,
              child: Column(
                mainAxisSize: .min,
                children: [
                  Text(
                    title,
                    style: titleStyle,
                    textAlign: .center,
                    maxLines: 1,
                    overflow: .ellipsis,
                  ),
                  if (subtitle case final subtitle?)
                    Text(
                      subtitle,
                      style: muted,
                      textAlign: .center,
                      maxLines: 1,
                      overflow: .ellipsis,
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

typedef _TaskRow = ({String label, int count, Duration age, bool waiting});

/// One row per label: how many run, the oldest one's age, and whether all
/// of them wait on a sign-in.
List<_TaskRow> _taskRows(GlobalAsyncQueue queue, List<String> tasks) {
  final now = DateTime.now();
  final rows = <String, _TaskRow>{};
  for (final id in tasks) {
    final age = now.difference(queue.startedAt(id) ?? now);
    final waiting = queue.isWaiting(id);
    rows.update(
      vtopTaskLabel(id),
      (row) => (
        label: row.label,
        count: row.count + 1,
        age: age > row.age ? age : row.age,
        waiting: row.waiting && waiting,
      ),
      ifAbsent: () =>
          (label: vtopTaskLabel(id), count: 1, age: age, waiting: waiting),
    );
  }
  return rows.values.toList();
}

/// The list shown while the status line is held; ticks every second.
class _RunningTasks extends HookWidget {
  const _RunningTasks({
    required this.queue,
    required this.tasks,
    required this.theme,
  });

  final GlobalAsyncQueue queue;
  final List<String> tasks;
  final FThemeData theme;

  @override
  Widget build(BuildContext context) {
    final tick = useState(0);
    useEffect(
      () => Timer.periodic(
        const Duration(seconds: 1),
        (_) => tick.value++,
      ).cancel,
      const [],
    );
    final rows = _taskRows(queue, tasks);
    final colors = theme.colors;
    final label = theme.typography.body.sm.copyWith(color: colors.foreground);
    final small = theme.typography.body.xs.copyWith(
      color: colors.mutedForeground,
      fontFeatures: const [FontFeature.tabularFigures()],
    );
    return Container(
      width: 260,
      decoration: BoxDecoration(
        color: colors.background,
        border: .all(color: colors.border),
        borderRadius: .circular(12),
      ),
      padding: const .symmetric(vertical: 4),
      child: Column(
        mainAxisSize: .min,
        children: [
          if (rows.isEmpty)
            Padding(
              padding: const .all(12),
              child: Text('Nothing running', style: small),
            ),
          for (final (i, row) in rows.indexed) ...[
            if (i > 0) Divider(height: 1, thickness: 1, color: colors.border),
            Opacity(
              opacity: row.waiting ? 0.5 : 1,
              child: Padding(
                padding: const .symmetric(horizontal: 12, vertical: 8),
                child: Row(
                  spacing: 8,
                  children: [
                    Expanded(
                      child: Text(
                        row.label,
                        style: label,
                        maxLines: 1,
                        overflow: .ellipsis,
                      ),
                    ),
                    if (row.count > 1) Text('×${row.count}', style: small),
                    // Each new value slides up into place.
                    AnimatedSwitcher(
                      duration: const Duration(milliseconds: 300),
                      transitionBuilder: (child, animation) => FadeTransition(
                        opacity: animation,
                        child: SlideTransition(
                          position: Tween(
                            begin: const Offset(0, 0.5),
                            end: Offset.zero,
                          ).animate(animation),
                          child: child,
                        ),
                      ),
                      child: Text(
                        row.waiting ? 'waiting' : _age(row.age),
                        key: ValueKey(row.waiting ? 'waiting' : _age(row.age)),
                        style: small,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

String _age(Duration age) =>
    age.inMinutes > 0 ? '${age.inMinutes}m' : '${age.inSeconds}s';

String? _queueStatusText(List<String> runningTasks) {
  if (runningTasks.isEmpty) return null;
  final visibleTasks = runningTasks.map(vtopTaskLabel).toSet();
  final extraCount = visibleTasks.length - 1;
  return extraCount <= 0
      ? visibleTasks.first
      : '${visibleTasks.first} + $extraCount more';
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
