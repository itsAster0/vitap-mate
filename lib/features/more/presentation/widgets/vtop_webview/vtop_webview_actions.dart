import 'package:flutter/material.dart';
import 'package:forui/forui.dart';

class VtopWebviewThemeAction extends StatelessWidget {
  const VtopWebviewThemeAction({
    required this.isDarkMode,
    required this.onToggle,
    super.key,
  });

  final bool isDarkMode;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    return FHeaderAction(
      icon: Icon(isDarkMode ? Icons.light_mode : Icons.dark_mode, size: 20),
      onPress: onToggle,
    );
  }
}

class VtopWebviewActionsMenu extends StatelessWidget {
  const VtopWebviewActionsMenu({
    required this.isCompactMode,
    required this.isDesktopMode,
    required this.onGoTo,
    required this.onToggleCompactMode,
    required this.onToggleDesktopMode,
    required this.onForceLogin,
    super.key,
  });

  final bool isCompactMode;
  final bool isDesktopMode;
  final ValueChanged<String> onGoTo;
  final VoidCallback onToggleCompactMode;
  final VoidCallback onToggleDesktopMode;
  final VoidCallback onForceLogin;

  @override
  Widget build(BuildContext context) {
    return FPopoverMenu(
      autofocus: true,
      menuAnchor: Alignment.topRight,
      childAnchor: Alignment.bottomRight,
      menu: [
        FItemGroup(
          children: [
            FItem(
              prefix: const Icon(FIcons.calendar),
              title: const Text('Timetable'),
              onPress: () => onGoTo('academics/common/StudentTimeTable'),
            ),
            FItem(
              prefix: const Icon(FIcons.paperclip),
              title: const Text('Attendance'),
              onPress: () => onGoTo('academics/common/StudentAttendance'),
            ),
            FItem(
              prefix: const Icon(FIcons.book),
              title: const Text('CoursePage'),
              onPress: () => onGoTo('academics/common/StudentCoursePage'),
            ),
            FItem(
              prefix: const Icon(FIcons.university),
              title: const Text('Academic Calendar'),
              onPress: () => onGoTo('academics/common/CalendarPreview'),
            ),
            FItem(
              prefix: const Icon(FIcons.workflow),
              title: const Text('Digital Assignment'),
              onPress: () => onGoTo('examinations/StudentDA'),
            ),
          ],
        ),
        FItemGroup(
          children: [
            FItem(
              prefix: const Icon(FIcons.graduationCap),
              title: const Text('Grades'),
              onPress:
                  () => onGoTo('examinations/examGradeView/StudentGradeView'),
            ),
            FItem(
              prefix: const Icon(FIcons.history),
              title: const Text('Grades History'),
              onPress:
                  () =>
                      onGoTo('examinations/examGradeView/StudentGradeHistory'),
            ),
          ],
        ),
        FItemGroup(
          children: [
            FItem(
              prefix: const Icon(FIcons.amphora),
              title: const Text('Weekend Outing'),
              onPress: () => onGoTo('hostel/StudentWeekendOuting'),
            ),
            FItem(
              prefix: const Icon(FIcons.anchor),
              title: const Text('General Outing'),
              onPress: () => onGoTo('hostel/StudentGeneralOuting'),
            ),
          ],
        ),
        FItemGroup(
          children: [
            FItem(
              prefix: FCheckbox(value: isCompactMode),
              title: const Text('Compact View'),
              onPress: onToggleCompactMode,
            ),
            FItem(
              prefix: FCheckbox(value: isDesktopMode),
              title: const Text('Desktop Mode'),
              onPress: onToggleDesktopMode,
            ),
            FItem(
              prefix: const Icon(FIcons.logIn),
              title: const Text('Force Login'),
              onPress: onForceLogin,
            ),
          ],
        ),
      ],
      builder:
          (_, controller, _) => FHeaderAction(
            icon: const Icon(FIcons.ellipsis),
            onPress: controller.toggle,
          ),
    );
  }
}
