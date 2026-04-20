import 'package:flutter/material.dart';

import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/sync_google_calendar_button.dart';

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
      if (sem == null || sems == null) return false;
      final max = sems.semesters.fold<int>(0, (i, e) {
        final result =
            int.tryParse(e.id.toLowerCase().replaceAll("ap", "")) ?? 0;
        return result > i ? result : i;
      });

      if (max == 0) return false;
      if (sem.semid == null) return false;

      final currentSemID =
          int.tryParse(sem.semid!.toLowerCase().replaceAll("ap", "")) ?? 0;
      return currentSemID < max;
    }, [sem?.semid, sems?.semesters.length]);
    final runningTasks = ref.watch(
      globalAsyncQueueProvider.select((value) => value.running.keys.toList()),
    );

    var k = GoRouter.of(context).routeInformationProvider.value.uri.toString();
    final headers = [
      getSidewidget(context, "Timetable", k, newSemExist, runningTasks),
      getSidewidget(context, "Attendance", k, newSemExist, runningTasks),
      getSidewidget(context, "More", k, newSemExist, runningTasks),
      getSidewidget(context, "Projects", k, newSemExist, runningTasks),
      getSidewidget(context, "Settings", k, newSemExist, runningTasks),
    ];
    final selected = useState(0);
    useEffect(() {
      if (k.startsWith("/timetable")) {
        selected.value = 0;
      } else if (k.startsWith("/attendance")) {
        selected.value = 1;
      } else if (k.startsWith("/more")) {
        selected.value = 2;
      } else if (k.startsWith("/student-projects")) {
        selected.value = 3;
      } else if (k.startsWith("/settings")) {
        selected.value = 4;
      }
      return null;
    }, [k]);

    return FScaffold(
      childPad: false,
      scaffoldStyle: FScaffoldStyleDelta.delta(
        footerDecoration: DecorationDelta.value(
          BoxDecoration(color: context.theme.colors.background),
        ),
      ),
      header: headers[selected.value],
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
            selected.value = index;
            switch (selected.value) {
              case 0:
                GoRouter.of(context).goNamed(Paths.timetable);
                break;
              case 1:
                GoRouter.of(context).goNamed(Paths.attendance);
                break;
              case 2:
                GoRouter.of(context).goNamed(Paths.more);
                break;
              // case 3:
              //   GoRouter.of(context).goNamed(Paths.studentProjects);
              //   break;
              case 3:
                GoRouter.of(context).goNamed(Paths.settings);
                break;
            }
          },
          children: [
            FBottomNavigationBarItem(
              icon: Icon(FIcons.calendarDays),
              label: const Text('Timetable'),
            ),
            FBottomNavigationBarItem(
              icon: Icon(FIcons.userCheck),
              label: const Text('Attendance'),
            ),
            FBottomNavigationBarItem(
              icon: Icon(FIcons.libraryBig),
              label: const Text('More'),
            ),
            // FBottomNavigationBarItem(
            //   icon: Icon(FIcons.rocket),
            //   label: const Text('Projects'),
            // ),
            FBottomNavigationBarItem(
              icon: Icon(FIcons.settings),
              label: const Text('Settings'),
            ),
          ],
        ),
      ),

      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8.0),
        child: child,
      ),
    );
  }
}

Widget? getSidewidget(
  BuildContext context,
  String data,
  String path,
  bool newsem,
  List<String> runningTasks,
) {
  final queueStatus = _queueStatusText(runningTasks);
  if (path.split('/').length - 1 > 1) {
    switch (path.split("/")[2]) {
      case "marks":
        data = "Marks";
        break;
      case "exam_schedule":
        data = "Exam Schedule";
        break;
    }

    return FHeader.nested(
      title: _HeaderTitle(
        title: data,
        subtitle: queueStatus,
        titleStyle: DefaultTextStyle.of(context).style,
      ),
      prefixes: [FHeaderAction.back(onPress: () => GoRouter.of(context).pop())],
    );
  }

  final subtitle =
      queueStatus ?? (newsem ? "New semester data available!" : null);
  final hasTimetableAction = path.contains("timetable");
  return FHeader.nested(
    title: _HeaderTitle(
      title: data,
      subtitle: subtitle,
      titleStyle: subtitle != null
          ? context.theme.typography.sm
          : context.theme.typography.lg,
    ),

    prefixes: [
      if (hasTimetableAction)
        Visibility(
          visible: false,
          maintainAnimation: true,
          maintainSize: true,
          maintainState: true,
          child: SyncGoogleCalendarButton(),
        ),
    ],

    suffixes: [if (hasTimetableAction) SyncGoogleCalendarButton()],
  );
}

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
              style: context.theme.typography.sm.copyWith(
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

  final visibleTasks = runningTasks.map(_taskLabel).toSet().toList();
  final firstTask = visibleTasks.first;
  final extraCount = visibleTasks.length - 1;

  if (extraCount <= 0) return firstTask;
  return '$firstTask + $extraCount more';
}

String _taskLabel(String id) {
  if (id.startsWith('vtop_login')) return 'Logging in to VTOP...';
  if (id.startsWith('vtop_attendance')) return 'Fetching attendance...';
  if (id.startsWith('vtop_fullattendance')) {
    return 'Fetching attendance details...';
  }
  if (id.startsWith('vtop_timetable')) return 'Fetching timetable...';
  if (id.startsWith('vtop_fetchSchedule')) return 'Fetching exam schedule...';
  if (id.startsWith('vtop_marks')) return 'Fetching marks...';
  if (id.startsWith('vtop_grades')) return 'Fetching grades...';
  if (id.startsWith('vtop_grade_details')) return 'Fetching grade details...';
  if (id.startsWith('vtop_grade_history')) return 'Fetching grade history...';
  if (id.startsWith('vtop_semids')) return 'Fetching semesters...';

  if (id.startsWith('toStorage')) return 'Saving data...';
  if (id.startsWith('fromStorage') || id == 'get_semids_storage') {
    return 'Loading data...';
  }

  return 'Working...';
}
