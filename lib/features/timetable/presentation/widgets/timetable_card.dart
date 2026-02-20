import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/timetable_colors.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum ClassStatus { completed, ongoing, upcoming, nextClass, notToday }

class TimetableCard extends HookConsumerWidget {
  final TimetableSlot slot;
  const TimetableCard({super.key, required this.slot});

  ClassStatus getClassStatus(
    String startTimeStr,
    String endTimeStr,
    String classWeekday,
  ) {
    final now = DateTime.now();
    final currentTime = Duration(hours: now.hour, minutes: now.minute);

    Duration parseTime(String timeStr) {
      final parts = timeStr.split(':').map(int.parse).toList();
      return Duration(hours: parts[0], minutes: parts[1]);
    }

    const weekdayMap = {
      'MON': DateTime.monday,
      'TUE': DateTime.tuesday,
      'WED': DateTime.wednesday,
      'THU': DateTime.thursday,
      'FRI': DateTime.friday,
      'SAT': DateTime.saturday,
      'SUN': DateTime.sunday,
    };

    final startTime = parseTime(startTimeStr);
    final endTime = parseTime(endTimeStr);
    final nextClassThreshold = currentTime + const Duration(minutes: 50);
    if (now.weekday != weekdayMap[classWeekday]) {
      return ClassStatus.notToday;
    }
    if (currentTime >= startTime && currentTime <= endTime) {
      return ClassStatus.ongoing;
    } else if (currentTime > endTime) {
      return ClassStatus.completed;
    } else if (nextClassThreshold >= startTime &&
        nextClassThreshold <= endTime) {
      return ClassStatus.nextClass;
    } else {
      return ClassStatus.upcoming;
    }
  }

  Color getCardBackgroundColor(bool isDark, BuildContext context) {
    if (isDark) return context.theme.colors.primaryForeground;
    if (slot.serial == "-1") return TimetableColors.freeTimeBackground;

    return slot.islab()
        ? TimetableColors.labBackground
        : TimetableColors.lectureBackground;
  }

  (Color border, Color background, Color text, String label) getStatusStyle(
    ClassStatus status,
  ) {
    switch (status) {
      case ClassStatus.ongoing:
        return (
          TimetableColors.ongoingBorder,
          TimetableColors.ongoingBackground,
          TimetableColors.ongoingText,
          'ONGOING',
        );
      case ClassStatus.completed:
        return (
          TimetableColors.completedBorder,
          TimetableColors.completedBackground,
          TimetableColors.completedText,
          'COMPLETED',
        );
      case ClassStatus.nextClass:
        return (
          TimetableColors.nextClassBorder,
          TimetableColors.nextClassBackground,
          TimetableColors.nextClassText,
          'NEXT CLASS',
        );
      case ClassStatus.upcoming:
        return (
          TimetableColors.upcomingBorder,
          TimetableColors.upcomingBackground,
          TimetableColors.upcomingText,
          'UPCOMING',
        );
      case ClassStatus.notToday:
        return (
          const Color.fromARGB(255, 86, 25, 183),
          TimetableColors.upcomingBackground,
          const Color.fromARGB(255, 99, 25, 210),
          'NOT TODAY',
        );
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final controller = useAnimationController(
      duration: const Duration(milliseconds: 300),
    );

    final animation = CurvedAnimation(
      parent: controller,
      curve: Curves.easeInOut,
    );

    final isExpanded = useState(false);

    final status =
        slot.serial != "-1"
            ? getClassStatus(slot.startTime, slot.endTime, slot.day)
            : null;

    final statusStyle = status != null ? getStatusStyle(status) : null;
    final isHighlighted =
        status == ClassStatus.ongoing || status == ClassStatus.nextClass;

    return GestureDetector(
      onTap: () {
        isExpanded.value = !isExpanded.value;
        isExpanded.value ? controller.forward() : controller.reverse();
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8.0, vertical: 4.0),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 300),
          decoration: BoxDecoration(
            color: getCardBackgroundColor(darkMode, context),
            borderRadius: BorderRadius.circular(12),
            border:
                statusStyle != null && isHighlighted
                    ? Border.all(color: statusStyle.$1, width: 2)
                    : null,
            boxShadow:
                darkMode
                    ? null
                    : [
                      BoxShadow(
                        color:
                            isHighlighted
                                ? TimetableColors.statusShadow
                                : TimetableColors.cardShadow,
                        blurRadius: isHighlighted ? 8 : 6,
                        offset: const Offset(0, 2),
                        spreadRadius: isHighlighted ? 1 : 0,
                      ),
                    ],
          ),
          child: Padding(
            padding: const EdgeInsets.all(12),
            child:
                slot.serial != "-1"
                    ? _buildClassCard(darkMode, context, statusStyle, animation)
                    : _buildFreeTimeCard(darkMode, context),
          ),
        ),
      ),
    );
  }

  Widget _buildClassCard(
    bool isDark,
    BuildContext context,
    (Color, Color, Color, String)? statusStyle,
    CurvedAnimation animation,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color:
                    slot.islab()
                        ? const Color(0xFFE1F5FE)
                        : const Color(0xFFE8F4FD),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    slot.islab() ? FIcons.flaskConical : FIcons.libraryBig,
                    size: 16,
                    color:
                        slot.islab()
                            ? const Color(0xFF0277BD)
                            : const Color(0xFF1565C0),
                  ),
                  const SizedBox(width: 4),
                  Text(
                    slot.islab() ? 'LAB' : 'LECTURE',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color:
                          slot.islab()
                              ? const Color(0xFF0277BD)
                              : const Color(0xFF1565C0),
                    ),
                  ),
                ],
              ),
            ),
            const Spacer(),
            if (statusStyle != null)
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: statusStyle.$2,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: statusStyle.$1, width: 1),
                ),
                child: Text(
                  statusStyle.$4,
                  style: TextStyle(
                    fontSize: 10,
                    fontWeight: FontWeight.bold,
                    color: statusStyle.$3,
                  ),
                ),
              ),
          ],
        ),
        const SizedBox(height: 8),

        Text(
          slot.name,
          style: TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w600,
            color: isDark ? context.theme.colors.primary : Color(0xFF374151),
            //height: 1.2,
          ),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 8),

        Row(
          children: [
            Expanded(
              child: _buildDetailChip(
                isDark,
                context,
                icon: FIcons.mapPin,
                text: "${slot.block} - ${slot.roomNo}",
                color:
                    isDark
                        ? context.theme.colors.primary
                        : const Color(0xFF6B7280),
              ),
            ),
            const SizedBox(width: 8),
            _buildDetailChip(
              isDark,
              context,
              icon: FIcons.hash,
              text: slot.courseCode,
              color:
                  isDark
                      ? context.theme.colors.primary
                      : const Color(0xFF6B7280),
            ),
          ],
        ),
        const SizedBox(height: 8),

        Row(
          children: [
            Expanded(
              child: _buildDetailChip(
                isDark,
                context,
                icon: FIcons.clock,
                text:
                    "${to12H(slot.startTime, context)} - ${to12H(slot.endTime, context)}",
                color:
                    isDark
                        ? context.theme.colors.primary
                        : const Color(0xFF374151),
                isBold: true,
              ),
            ),
            const SizedBox(width: 8),
            _buildDetailChip(
              isDark,
              context,
              icon: FIcons.calendar,
              text: slot.slot,
              color:
                  isDark
                      ? context.theme.colors.primary
                      : const Color(0xFF6B7280),
            ),
          ],
        ),

        SizeTransition(
          sizeFactor: animation,
          axisAlignment: -1.0,
          child: Column(
            children: [
              const SizedBox(height: 8),
              Row(
                children: [
                  Expanded(
                    child: _buildDetailChip(
                      isDark,
                      context,
                      icon: FIcons.contact,
                      text: formateFaculityName(slot.faculty),
                      color:
                          isDark
                              ? context.theme.colors.primary
                              : const Color(0xFF374151),
                      isBold: true,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildFreeTimeCard(bool isDark, BuildContext context) {
    return Column(
      children: [
        Row(
          children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color: const Color(0xFFFFF8E1),
                borderRadius: BorderRadius.circular(12),
              ),
              child: const Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(FIcons.hourglass, size: 16, color: Color(0xFFE65100)),
                  SizedBox(width: 4),
                  Text(
                    'FREE TIME',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: Color(0xFFE65100),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),

        Row(
          children: [
            Expanded(
              child: _buildDetailChip(
                isDark,
                context,
                icon: FIcons.clock,
                text:
                    "${to12H(slot.startTime, context)} - ${to12H(slot.endTime, context)}",
                color:
                    isDark
                        ? context.theme.colors.primary
                        : const Color(0xFF374151),
                isBold: true,
              ),
            ),
            const SizedBox(width: 8),
            _buildDetailChip(
              isDark,
              context,
              icon: FIcons.timer,
              text:
                  slot.slot == "1" ? "${slot.slot} hour" : "${slot.slot} hours",
              color:
                  isDark
                      ? context.theme.colors.primary
                      : const Color(0xFF6B7280),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildDetailChip(
    bool isDark,
    BuildContext context, {
    required IconData icon,
    required String text,
    required Color color,
    bool isBold = false,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: (isDark ? Colors.black : context.theme.colors.primaryForeground)
            .withValues(alpha: 0.7),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 4),
          Flexible(
            child: Text(
              text,
              style: TextStyle(
                fontSize: 13,
                fontWeight: isBold ? FontWeight.w600 : FontWeight.w400,
                color: color,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

String to12H(String time, BuildContext context) {
  if (MediaQuery.of(context).alwaysUse24HourFormat) {
    return time;
  }

  var list = time.split(":");
  int hours = int.parse(list[0]);
  String period;

  if (hours > 12) {
    hours -= 12;
    period = "PM";
  } else if (hours == 12) {
    period = "PM";
  } else if (hours == 0) {
    hours = 12;
    period = "AM";
  } else {
    period = "AM";
  }

  return "$hours:${list[1]} $period";
}

String formateFaculityName(String value) {
  try {
    String delimiter = "-";
    int lastIndex = value.lastIndexOf(delimiter);
    String partBefore = value.substring(0, lastIndex);
    return partBefore;
  } catch (_) {
    return value;
  }
}
