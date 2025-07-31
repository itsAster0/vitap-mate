import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
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

    return Container(
      height: double.infinity,
      width: double.infinity,
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      ),
      child: Column(
        children: [
          _buildHeader(context, facultyName),
          Expanded(
            child: dataAsync.when(
              data: (data) => _buildTableContent(context, data),
              error: (e, se) => _buildErrorState(e),
              loading: () => _buildLoadingState(),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHeader(BuildContext context, String name) {
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
          Text(
            "by $name",
            style: const TextStyle(
              fontSize: 14,
              fontWeight: FontWeight.w700,
              color: AttendanceColors.primaryText,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTableContent(BuildContext context, dynamic data) {
    if (data.records.isEmpty) {
      return _buildEmptyState();
    }

    return Column(
      children: [
        Expanded(
          child: Container(
            margin: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: AttendanceColors.tableBackground,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.grey.shade200),
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
                    dividerThickness: 1,
                    headingRowColor: WidgetStateProperty.all(
                      AttendanceColors.tableHeaderBackground,
                    ),
                    headingRowHeight: 56,
                    dataRowMinHeight: 48,
                    dataRowMaxHeight: 64,
                    columnSpacing: 24,
                    horizontalMargin: 16,
                    columns: [
                      _buildDataColumn("#", 40),
                      _buildDataColumn("Date", 100),
                      _buildDataColumn("Status", 80),
                      _buildDataColumn("Time", 80),
                      _buildDataColumn("Slot", 60),
                      _buildDataColumn("Remark", 120),
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
                                  : AttendanceColors.tableRowAlternate,
                            ),
                            cells: [
                              _buildDataCell(record.serial, isNumeric: true),
                              _buildDataCell(_formatDate(record.date)),
                              DataCell(_buildStatusBadge(record.status)),
                              _buildDataCell(record.dayTime),
                              _buildDataCell(record.slot),
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
                                      style: const TextStyle(
                                        fontSize: 13,
                                        color: AttendanceColors.secondaryText,
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

  DataColumn _buildDataColumn(String label, double width) {
    return DataColumn(
      label: SizedBox(
        width: width,
        child: Text(
          label,
          style: const TextStyle(
            fontWeight: FontWeight.w600,
            fontSize: 14,
            color: AttendanceColors.primaryText,
          ),
        ),
      ),
    );
  }

  DataCell _buildDataCell(String text, {bool isNumeric = false}) {
    return DataCell(
      Text(
        text,
        style: TextStyle(
          fontSize: 13,
          fontWeight: isNumeric ? FontWeight.w600 : FontWeight.w400,
          color: AttendanceColors.secondaryText,
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
