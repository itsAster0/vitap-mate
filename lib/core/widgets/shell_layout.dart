import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/features/social/presentation/widgets/logout_button.dart';

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
      return null;
    }, [selected.value]);

    return FScaffold(
      header: headers[selected.value],
      footer: FBottomNavigationBar(
        index: selected.value,
        onChange: (index) => selected.value = index,
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
    prefixes: [],
    suffixes: [if (path.contains("social")) LogoutButton()],
  );
}
