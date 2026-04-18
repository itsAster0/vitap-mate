import 'package:flutter/material.dart';

import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
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

    var k = GoRouter.of(context).routeInformationProvider.value.uri.toString();
    final headers = [
      getSidewidget(context, "Timetable", k, newSemExist),
      getSidewidget(context, "Attendance", k, newSemExist),
      getSidewidget(context, "More", k, newSemExist),
      getSidewidget(context, "Projects", k, newSemExist),
      getSidewidget(context, "Settings", k, newSemExist),
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
      header: headers[selected.value],
      footer: SafeArea(
        top: false,
        right: false,
        left: false,
        bottom: true,
        child: FBottomNavigationBar(
          index: selected.value,
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
              case 3:
                GoRouter.of(context).goNamed(Paths.studentProjects);
                break;
              case 4:
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
            FBottomNavigationBarItem(
              icon: Icon(FIcons.rocket),
              label: const Text('Projects'),
            ),
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
) {
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
      title: Text(data),
      prefixes: [FHeaderAction.back(onPress: () => GoRouter.of(context).pop())],
    );
  }

  return FHeader.nested(
    title: Column(
      children: [
        Text(
          data,
          style: newsem
              ? context.theme.typography.sm
              : context.theme.typography.lg,
        ),
        if (newsem)
          Text(
            "New semester data available!",
            style: context.theme.typography.sm,
          ),
      ],
    ),

    prefixes: [],

    suffixes: [if (path.contains("timetable")) SyncGoogleCalendarButton()],
  );
}
