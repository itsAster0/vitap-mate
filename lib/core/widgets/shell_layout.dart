import 'package:flutter/material.dart';

import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/features/attendance/presentation/pages/attendance_page.dart';
import 'package:vitapmate/features/social/presentation/widgets/logout_button.dart';
import 'package:vitapmate/features/timetable/presentation/pages/share_tt.dart';

class ShellLayout extends HookWidget {
  final Widget child;
  const ShellLayout({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    var k = GoRouter.of(context).routeInformationProvider.value.uri.toString();
    final headers = [
      getSidewidget(context, "Timetable", k),
      getSidewidget(context, "Attendance", k),
      getSidewidget(context, "More", k),
      getSidewidget(context, "Social", k),
      getSidewidget(context, "Settings", k),
    ];
    final selected = useState(0);
    useEffect(() {
      if (k.startsWith("/timetable")) {
        selected.value = 0;
      } else if (k.startsWith("/attendance")) {
        selected.value = 1;
      } else if (k.startsWith("/more")) {
        selected.value = 2;
      } else if (k.startsWith("/social")) {
        selected.value = 3;
      } else if (k.startsWith("/settings")) {
        selected.value = 4;
      }
      return null;
    }, [k]);

    return FScaffold(
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
                GoRouter.of(context).goNamed(Paths.social);
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
              icon: Icon(FIcons.atSign),
              label: const Text('social'),
            ),
            FBottomNavigationBarItem(
              icon: Icon(FIcons.settings),
              label: const Text('Settings'),
            ),
          ],
        ),
      ),

      child: child,
    );
  }
}

Widget? getSidewidget(BuildContext context, String data, String path) {
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
    title: Text(data),
    prefixes: [if (path.contains("social")) InfoSocial()],
    suffixes: [
      if (path.contains("social")) LogoutButton(),
      if (path.contains("timetable")) ShareTt(),
      if (data == "Attendance") AttendanceHeader(),
    ],
  );
}

class InfoSocial extends StatelessWidget {
  const InfoSocial({super.key});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: FTappable(
        onPress:
            () => showAdaptiveDialog(
              context: context,
              builder:
                  (context) => FDialog(
                    direction: Axis.horizontal,
                    title: const Text('What information is shown?'),
                    body: const Text(
                      'Only your name and avatar are visible to others. You can change both anytime in Settings. '
                      'Your email remains private and is never shared.',
                    ),
                    actions: [
                      FButton(
                        onPress: () => Navigator.of(context).pop(),
                        child: const Text('Ok'),
                      ),
                    ],
                  ),
            ),
        child: const Icon(FIcons.info),
      ),
    );
  }
}
