import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/attendance/presentation/providers/full_attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_colors.dart';

class AttendanceTable extends HookConsumerWidget {
  final String courseId;
  final String courseType;
  final bool exp;
  final String facultyName;

  const AttendanceTable({
    super.key,
    required this.courseId,
    required this.courseType,
    required this.exp,
    required this.facultyName,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dataAsync = ref.watch(FullAttendanceProvider(courseType, courseId));
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final isLoading = useState(false);
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref
              .read(FullAttendanceProvider(courseType, courseId).notifier)
              .updateAttendance();
        } catch (e, _) {
          ();
        }
      });
      return null;
    }, []);

    void handelClick() async {
      isLoading.value = true;
      try {
        await ref
            .read(FullAttendanceProvider(courseType, courseId).notifier)
            .updateAttendance();
      } catch (e, _) {
        if (context.mounted) disCommonToast(context, e);
      } finally {
        isLoading.value = false;
      }
    }

    return Container(
      height: double.infinity,
      width: double.infinity,
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      ),
      child: Column(
        children: [
          _buildHeader(darkMode, context, facultyName, isLoading, handelClick),
          Expanded(
            child: dataAsync.when(
              data: (data) => _buildTableContent(darkMode, context, data),
              error: (e, se) => _buildErrorState(e),
              loading: () => _buildLoadingState(),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHeader(
    bool isDark,
    BuildContext context,
    String name,
    ValueNotifier<bool> isLoading,
    VoidCallback callback,
  ) {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
        border: Border(bottom: BorderSide(color: context.theme.colors.border)),
      ),
      child: Row(
        children: [
          Container(
            decoration: BoxDecoration(
              color: AttendanceColors.theoryIcon.withValues(alpha: 0.1),
            ),
            child: const Icon(
              Icons.table_chart_rounded,
              color: AttendanceColors.theoryIcon,
              size: 20,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              name,
              style: TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w700,
                color:
                    isDark
                        ? context.theme.colors.primary
                        : AttendanceColors.primaryText,
              ),
            ),
          ),
          if (!isLoading.value)
            FTappable(onPress: callback, child: Icon(FIcons.rotateCcw)),
          if (isLoading.value) FCircularProgress.pinwheel(),
          SizedBox(width: 10),
        ],
      ),
    );
  }

  Widget _buildTableContent(bool isDark, BuildContext context, dynamic data) {
    if (data.records.isEmpty) {
      return _buildEmptyState();
    }

    return Column(
      children: [
        Expanded(
          child: Container(
            margin: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color:
                  isDark
                      ? context.theme.colors.primaryForeground
                      : AttendanceColors.tableBackground,
              borderRadius: BorderRadius.circular(12),

              boxShadow: [
                BoxShadow(
                  color: AttendanceColors.cardShadowSecondary,
                  blurRadius: 8,
                  offset: const Offset(0, 2),
                ),
              ],
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(12),
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: SingleChildScrollView(
                  child: DataTable(
                    decoration: const BoxDecoration(),
                    dividerThickness: isDark ? 0 : 1,
                    headingRowColor: WidgetStateProperty.all(
                      isDark
                          ? context.theme.colors.primaryForeground
                          : AttendanceColors.tableHeaderBackground,
                    ),
                    headingRowHeight: 56,
                    dataRowMinHeight: 48,
                    dataRowMaxHeight: 64,
                    columnSpacing: 24,
                    horizontalMargin: 16,
                    columns: [
                      _buildDataColumn(
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                        "#",
                        40,
                      ),
                      _buildDataColumn(
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                        "Date",
                        100,
                      ),
                      _buildDataColumn(
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                        "Status",
                        80,
                      ),
                      _buildDataColumn(
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                        "Time",
                        80,
                      ),
                      _buildDataColumn(
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                        "Slot",
                        60,
                      ),
                      _buildDataColumn(
                        isDark
                            ? context.theme.colors.primary
                            : AttendanceColors.secondaryText,
                        "Remark",
                        120,
                      ),
                    ],
                    rows:
                        data.records.asMap().entries.map<DataRow>((entry) {
                          final index = entry.key;
                          final record = entry.value;
                          final isEven = index % 2 == 0;

                          return DataRow(
                            color: WidgetStateProperty.all(
                              isEven
                                  ? Colors.transparent
                                  : isDark
                                  ? context.theme.colors.primaryForeground
                                  : AttendanceColors.tableRowAlternate,
                            ),
                            cells: [
                              _buildDataCell(
                                isDark
                                    ? context.theme.colors.primary
                                    : AttendanceColors.secondaryText,
                                record.serial,
                                isNumeric: true,
                              ),
                              _buildDataCell(
                                isDark
                                    ? context.theme.colors.primary
                                    : AttendanceColors.secondaryText,
                                _formatDate(record.date),
                              ),
                              DataCell(_buildStatusBadge(record.status)),
                              _buildDataCell(
                                isDark
                                    ? context.theme.colors.primary
                                    : AttendanceColors.secondaryText,
                                record.dayTime,
                              ),
                              _buildDataCell(
                                isDark
                                    ? context.theme.colors.primary
                                    : AttendanceColors.secondaryText,
                                record.slot,
                              ),
                              DataCell(
                                Container(
                                  constraints: const BoxConstraints(
                                    maxWidth: 120,
                                  ),
                                  child: Tooltip(
                                    message: record.remark,
                                    child: Text(
                                      record.remark,
                                      overflow: TextOverflow.ellipsis,
                                      maxLines: 2,
                                      style: TextStyle(
                                        fontSize: 13,
                                        color:
                                            isDark
                                                ? context.theme.colors.primary
                                                : AttendanceColors
                                                    .secondaryText,
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ],
                          );
                        }).toList(),
                  ),
                ),
              ),
            ),
          ),
        ),
        Text(
          "Data updated on ${formatUnixTimestamp(data.updateTime.toInt())}",
          style: const TextStyle(
            fontSize: 12,
            color: AttendanceColors.tertiaryText,
            fontStyle: FontStyle.italic,
          ),
        ),
      ],
    );
  }

  DataColumn _buildDataColumn(Color color, String label, double width) {
    return DataColumn(
      label: SizedBox(
        width: width,
        child: Text(
          label,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            fontSize: 14,
            color: color,
          ),
        ),
      ),
    );
  }

  DataCell _buildDataCell(Color color, String text, {bool isNumeric = false}) {
    return DataCell(
      Text(
        text,
        style: TextStyle(
          fontSize: 13,
          fontWeight: isNumeric ? FontWeight.w600 : FontWeight.w400,
          color: color,
        ),
      ),
    );
  }

  Widget _buildStatusBadge(String status) {
    final statusLower = status.toLowerCase();

    Color backgroundColor;
    Color textColor;
    Color borderColor;
    IconData icon;

    switch (statusLower) {
      case 'absent':
        backgroundColor = AttendanceColors.absentBackground;
        textColor = AttendanceColors.absentText;
        borderColor = AttendanceColors.absentBorder;
        icon = Icons.close_rounded;
        break;
      case 'present':
        backgroundColor = AttendanceColors.presentBackground;
        textColor = AttendanceColors.presentText;
        borderColor = AttendanceColors.presentBorder;
        icon = Icons.check_rounded;
        break;
      default:
        backgroundColor = AttendanceColors.unknownBackground;
        textColor = AttendanceColors.unknownText;
        borderColor = AttendanceColors.unknownBorder;
        icon = Icons.help_outline_rounded;
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: borderColor, width: 1),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 12, color: textColor),
          const SizedBox(width: 4),
          Text(
            status,
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: textColor,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: AttendanceColors.tableRowAlternate,
              borderRadius: BorderRadius.circular(16),
            ),
            child: const Icon(
              Icons.event_busy_rounded,
              size: 48,
              color: AttendanceColors.tertiaryText,
            ),
          ),
          const SizedBox(height: 16),
          const Text(
            "No attendance data available",
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: AttendanceColors.secondaryText,
            ),
          ),
          const SizedBox(height: 8),
          const Text(
            "Check back later for updates",
            style: TextStyle(
              fontSize: 14,
              color: AttendanceColors.tertiaryText,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildErrorState(dynamic error) {
    final message = commonErrorMessage(error);
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: AttendanceColors.criticalBackground,
              borderRadius: BorderRadius.circular(16),
            ),
            child: const Icon(
              Icons.error_outline_rounded,
              size: 48,
              color: AttendanceColors.criticalText,
            ),
          ),
          const SizedBox(height: 16),
          const Text(
            "Unable to load data",
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: AttendanceColors.secondaryText,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            message,
            style: const TextStyle(
              fontSize: 14,
              color: AttendanceColors.tertiaryText,
            ),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }

  Widget _buildLoadingState() {
    return const Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          SizedBox(
            width: 40,
            height: 40,
            child: CircularProgressIndicator(
              color: AttendanceColors.theoryIcon,
              strokeWidth: 3,
            ),
          ),
          SizedBox(height: 16),
          Text(
            "Loading attendance data...",
            style: TextStyle(
              fontSize: 14,
              color: AttendanceColors.secondaryText,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }

  String _formatDate(String date) {
    try {
      final parts = date.split('-');
      if (parts.length == 3) {
        final day = parts[0];
        final month = parts[1];
        final year = parts[2];

        const months = [
          '',
          'Jan',
          'Feb',
          'Mar',
          'Apr',
          'May',
          'Jun',
          'Jul',
          'Aug',
          'Sep',
          'Oct',
          'Nov',
          'Dec',
        ];

        final monthIndex = int.tryParse(month);
        if (monthIndex != null && monthIndex >= 1 && monthIndex <= 12) {
          return '$day ${months[monthIndex]}';
        }
      }
    } catch (e) {
      ();
    }
    return date;
  }
}
