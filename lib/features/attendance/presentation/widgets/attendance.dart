import 'dart:math';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_colors.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_table.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AttendanceCard extends HookConsumerWidget {
  final AttendanceRecord record;
  final int index;

  const AttendanceCard({super.key, required this.record, required this.index});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isPressed = useState(false);
    final animationController = useAnimationController(
      duration: const Duration(milliseconds: 200),
    );
    final scaleAnimation = Tween<double>(begin: 1.0, end: 0.95).animate(
      CurvedAnimation(parent: animationController, curve: Curves.easeInOut),
    );
    final darkMode = ref.watch(themeControllerProvider) == ThemeMode.dark;
    final btwExams = ref.watch(btwExamsProvider);

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: AnimatedBuilder(
        animation: scaleAnimation,
        builder:
            (context, child) => Transform.scale(
              scale: scaleAnimation.value,
              child: GestureDetector(
                onTapDown: (_) {
                  isPressed.value = true;
                  animationController.forward();
                },
                onTapUp: (_) {
                  isPressed.value = false;
                  animationController.reverse();
                },
                onTapCancel: () {
                  isPressed.value = false;
                  animationController.reverse();
                },
                onTap: () => _showAttendanceDetails(context),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(8),
                    border: Border(
                      left: BorderSide(
                        color:
                            record.islab()
                                ? AttendanceColors.labIcon
                                : AttendanceColors.theoryIcon,
                        width: 4,
                      ),
                    ),
                  ),
                  child: FCard(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        _buildHeader(darkMode, context, btwExams),
                        const SizedBox(height: 16),
                        _buildStatsRow(darkMode, context, btwExams),
                      ],
                    ),
                  ),
                ),
              ),
            ),
      ),
    );
  }

  Widget _buildHeader(bool isDark, BuildContext context, bool btwExam) {
    final (courseCode, courseName) = formateName(record.courseName);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _buildCourseIcon(isDark, context),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (courseCode.isNotEmpty) ...[
                Text(
                  courseCode,
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    color:
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                    letterSpacing: 0.5,
                  ),
                ),
                const SizedBox(height: 2),
              ],
              Text(
                courseName.isNotEmpty ? courseName : record.courseName,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontWeight: FontWeight.w700,
                  fontSize: 16,
                  height: 1.3,
                  color:
                      isDark
                          ? context.theme.colors.primary
                          : AttendanceColors.primaryText,
                ),
              ),
            ],
          ),
        ),
        const SizedBox(width: 8),
        _buildAttendanceIndicator(isDark, context, btwExam),
      ],
    );
  }

  Widget _buildCourseIcon(bool isDark, BuildContext conntext) {
    final isLab = record.islab();

    final iconColor =
        isLab ? AttendanceColors.labIcon : AttendanceColors.theoryIcon;

    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: conntext.theme.colors.primaryForeground.withValues(alpha: 0.9),
        borderRadius: BorderRadius.circular(12),
        boxShadow: [
          BoxShadow(
            color: iconColor.withValues(alpha: 0.1),
            blurRadius: 8,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      child: Icon(
        isLab ? FIcons.flaskConical : FIcons.libraryBig,
        size: 20,
        color: iconColor,
      ),
    );
  }

  Widget _buildAttendanceIndicator(
    bool isDark,
    BuildContext context,
    bool btwExams,
  ) {
    if (record.attendancePercentage == "-") {
      return _buildIndicatorChip("N/A", AttendanceColors.unknownText);
    }

    final percentage = _calculateMaxPercentage(btwExams);
    final (color, bgColor) = _getAttendanceColors(percentage);

    return _buildIndicatorChip("${percentage.toInt()}%", color, bgColor);
  }

  Widget _buildIndicatorChip(
    String text,
    Color color, [
    Color? backgroundColor,
  ]) {
    return FBadge(
      style: FBadgeStyleDelta.delta(
        decoration: DecorationDelta.value(
          BoxDecoration(
            color: backgroundColor ?? color.withValues(alpha: 0.1),
            border: Border.all(color: color.withValues(alpha: 0.3)),
            borderRadius: BorderRadius.circular(8),
          ),
        ),
        contentStyle: FBadgeContentStyleDelta.delta(
          labelTextStyle: TextStyleDelta.delta(
            fontSize: 12,
            fontWeight: FontWeight.w700,
            color: color,
          ),
        ),
      ),
      child: Text(text),
    );
  }

  double _calculateMaxPercentage(bool btwExams) {
    final normalattp =
        double.tryParse(record.attendancePercentage.replaceAll("%", "")) ?? 0;
    if (btwExams) {
      return max(
        normalattp,
        double.tryParse(record.attendenceFatCat.replaceAll("%", "")) ?? 0,
      );
    }
    return normalattp;
  }

  (Color, Color) _getAttendanceColors(double percentage) {
    if (percentage >= 75) {
      return (
        AttendanceColors.excellentText,
        AttendanceColors.excellentBackground,
      );
    } else if (percentage >= 70) {
      return (AttendanceColors.goodText, AttendanceColors.goodBackground);
    } else {
      return (AttendanceColors.warningText, AttendanceColors.warningBackground);
    }
  }

  Widget _buildStatsRow(bool isDark, BuildContext context, bool btwExams) {
    if (!btwExams || record.attendancePercentage == "-") {
      return Row(
        children: [
          Expanded(
            child: _buildInlineStatItem(
              isDark,
              context,
              "Total",
              record.attendancePercentage,
              FIcons.percent,
              AttendanceColors.totalStatColor,
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: _buildInlineStatItem(
              isDark,
              context,
              "Present",
              "${record.classesAttended}/${record.totalClasses}",
              FIcons.check,
              AttendanceColors.presentStatColor,
            ),
          ),
        ],
      );
    }

    final stats = <Widget>[
      _buildStatItem(
        isDark,
        context,
        "Total",
        record.attendancePercentage,
        FIcons.percent,
        AttendanceColors.totalStatColor,
      ),
    ];

    if (record.attendancePercentage != "-" && btwExams) {
      stats.add(
        _buildStatItem(
          isDark,
          context,
          "b/w exams",
          record.attendenceFatCat,
          FIcons.calendar,
          AttendanceColors.examStatColor,
        ),
      );
    }

    stats.add(
      _buildStatItem(
        isDark,
        context,
        "Present",
        "${record.classesAttended}/${record.totalClasses}",
        FIcons.check,
        AttendanceColors.presentStatColor,
      ),
    );

    return Row(
      children:
          stats
              .map((stat) => Expanded(child: stat))
              .expand(
                (widget) => [
                  widget,
                  if (widget != stats.last) const SizedBox(width: 8),
                ],
              )
              .take(stats.length * 2 - 1)
              .toList(),
    );
  }

  Widget _buildStatItem(
    bool isDark,
    BuildContext context,
    String label,
    String value,
    IconData icon,
    Color accentColor,
  ) {
    return FCard(
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Container(
              padding: const EdgeInsets.all(6),
              decoration: BoxDecoration(
                color: accentColor.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(icon, size: 16, color: accentColor),
            ),
            const SizedBox(height: 8),
            Text(
              value,
              style: TextStyle(
                fontSize: 15,
                fontWeight: FontWeight.w700,
                height: 1.2,
                color:
                    isDark
                        ? context.theme.colors.primary
                        : AttendanceColors.primaryText,
              ),
              textAlign: TextAlign.center,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            const SizedBox(height: 2),
            Text(
              label,
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w500,
                color:
                    isDark
                        ? context.theme.colors.primary
                        : AttendanceColors.secondaryText,
                height: 1.2,
              ),
              textAlign: TextAlign.center,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildInlineStatItem(
    bool isDark,
    BuildContext context,
    String label,
    String value,
    IconData icon,
    Color accentColor,
  ) {
    return FCard(
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(6),
            decoration: BoxDecoration(
              color: accentColor.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Icon(icon, size: 16, color: accentColor),
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.center,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  value,
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w700,
                    height: 1.2,
                    color:
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.primaryText,
                  ),
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                const SizedBox(height: 2),
                Text(
                  label,
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w500,
                    color:
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                    height: 1.2,
                  ),
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  void _showAttendanceDetails(BuildContext context) {
    showFSheet(
      mainAxisMaxRatio: 5 / 6,
      context: context,
      side: FLayout.btt,
      builder:
          (context) => AttendanceTable(
            courseId: record.courseId,
            courseType: record.courseType,
            exp: true,
            facultyName: record.facultyDetail,
          ),
    );
  }
}

(String, String) formateName(String name) {
  final splitName = name.split("-");
  if (splitName.length < 2) return ("", name);
  var nName = splitName[1];
  if (splitName.length > 3) {
    nName += "-${splitName[2]}";
  }

  return (splitName[0], nName);
}
