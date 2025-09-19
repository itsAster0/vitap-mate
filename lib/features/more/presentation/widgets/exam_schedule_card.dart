import 'package:flutter/material.dart';
import 'package:forui/theme.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamScheduleCard extends ConsumerWidget {
  final PerExamScheduleRecord record;
  const ExamScheduleCard({super.key, required this.record});

  int _getUpcomingExamsCount() {
    final now = DateTime.now();
    return record.records.where((exam) {
      try {
        final examDate = DateTime.parse(exam.examDate);
        return examDate.isAfter(now);
      } catch (e) {
        return false;
      }
    }).length;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final upcomingCount = _getUpcomingExamsCount();

    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;

    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: Container(
        decoration: BoxDecoration(
          gradient:
              darkMode
                  ? null
                  : const LinearGradient(
                    colors: [
                      ExamColors.examCardBackground,
                      ExamColors.examCardSecondary,
                    ],
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                  ),
          color: darkMode ? context.theme.colors.primaryForeground : null,
          borderRadius: BorderRadius.circular(16),
          boxShadow: [
            BoxShadow(
              color: ExamColors.cardShadow,
              blurRadius: 12,
              offset: const Offset(0, 4),
            ),
          ],
          border: Border.all(
            color: context.theme.colors.primaryForeground.withValues(
              alpha: 0.8,
            ),
            width: 1,
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: context.theme.colors.primaryForeground.withValues(
                  alpha: 0.9,
                ),
                borderRadius: const BorderRadius.vertical(
                  top: Radius.circular(16),
                ),
                border: Border(
                  bottom: BorderSide(
                    color: ExamColors.examIcon.withValues(alpha: 0.2),
                    width: 1,
                  ),
                ),
              ),
              child: Column(
                children: [
                  Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.all(10),
                        decoration: BoxDecoration(
                          color:
                              darkMode
                                  ? context.theme.colors.primaryForeground
                                  : ExamColors.todayBackground,
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(
                            color: ExamColors.todayBorder.withValues(
                              alpha: 0.3,
                            ),
                            width: 1,
                          ),
                        ),
                        child: Icon(
                          Icons.schedule,
                          color: ExamColors.examIcon,
                          size: 20,
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          record.examType,
                          style: TextStyle(
                            fontWeight: FontWeight.w700,
                            fontSize: 18,
                            color:
                                darkMode
                                    ? context.theme.colors.primary
                                    : ExamColors.primaryText,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      _buildInfoChip(
                        icon: Icons.assignment_outlined,
                        label: "${record.records.length} total",
                        color:
                            darkMode
                                ? context.theme.colors.primary
                                : ExamColors.secondaryText,
                      ),
                      const SizedBox(width: 8),
                      if (upcomingCount > 0)
                        _buildInfoChip(
                          icon: Icons.upcoming_outlined,
                          label: "$upcomingCount upcoming",
                          color: ExamColors.upcomingText,
                        ),
                    ],
                  ),
                ],
              ),
            ),
            Container(
              decoration: BoxDecoration(
                color:
                    darkMode
                        ? context.theme.colors.primaryForeground
                        : ExamColors.tableBackground,
              ),
              child: ClipRRect(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: DataTable(
                    decoration: const BoxDecoration(),
                    dividerThickness: darkMode ? 0 : 1,
                    headingRowColor: WidgetStateProperty.all(
                      darkMode
                          ? context.theme.colors.primaryForeground
                          : ExamColors.tableHeaderBackground,
                    ),
                    headingRowHeight: 56,
                    dataRowMinHeight: 48,
                    dataRowMaxHeight: 64,
                    columnSpacing: 16,
                    horizontalMargin: 16,
                    columns: [
                      _buildDataColumn(context, ""),
                      _buildDataColumn(context, "Course"),
                      _buildDataColumn(context, "Date"),
                      _buildDataColumn(context, "Venue"),
                      _buildDataColumn(context, "Seat"),
                      _buildDataColumn(context, "Seat No."),
                      _buildDataColumn(context, "Time"),
                      _buildDataColumn(context, "Report Time"),
                      _buildDataColumn(context, "Session"),
                      _buildDataColumn(context, "Slot"),
                      _buildDataColumn(context, "Course Type"),
                      _buildDataColumn(context, "Course Name"),
                      _buildDataColumn(context, "Class ID"),
                    ],
                    rows:
                        record.records.asMap().entries.map<DataRow>((entry) {
                          final index = entry.key;
                          final exam = entry.value;
                          final isEven = index % 2 == 0;
                          final isUpcoming = _isUpcomingExam(exam.examDate);

                          return DataRow(
                            color: WidgetStateProperty.all(
                              darkMode
                                  ? context.theme.colors.primaryForeground
                                  : isEven
                                  ? Colors.transparent
                                  : ExamColors.tableRowAlternate,
                            ),
                            cells: [
                              _buildDataCell(
                                context,
                                exam.serial,
                                isNumeric: true,
                              ),
                              _buildDataCell(context, exam.courseCode),
                              _buildDataCell(
                                context,
                                exam.examDate,
                                isDate: true,
                                isUpcoming: isUpcoming,
                              ),
                              _buildDataCell(context, exam.venue),
                              _buildDataCell(context, exam.seatLocation),
                              _buildDataCell(context, exam.seatNo),
                              _buildDataCell(context, exam.examTime),
                              _buildDataCell(context, exam.reportingTime),
                              _buildDataCell(context, exam.examSession),
                              _buildDataCell(context, exam.slot),
                              _buildDataCell(context, exam.courseType),
                              _buildDataCell(context, exam.courseName),
                              _buildDataCell(context, exam.courseId),
                            ],
                          );
                        }).toList(),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildInfoChip({
    required IconData icon,
    required String label,
    required Color color,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: .3)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 4),
          Text(
            label,
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w500,
              color: color,
            ),
          ),
        ],
      ),
    );
  }

  bool _isUpcomingExam(String examDate) {
    try {
      final date = DateTime.parse(examDate);
      return date.isAfter(DateTime.now());
    } catch (e) {
      return false;
    }
  }

  DataColumn _buildDataColumn(BuildContext context, String label) {
    return DataColumn(
      label: Text(
        label,
        style: TextStyle(
          fontWeight: FontWeight.w600,
          fontSize: 14,
          color: context.theme.colors.primary,
        ),
      ),
    );
  }

  DataCell _buildDataCell(
    BuildContext context,
    String text, {
    bool isNumeric = false,
    bool isDate = false,
    bool isUpcoming = false,
  }) {
    return DataCell(
      Container(
        constraints: const BoxConstraints(maxWidth: 120),
        child: Text(
          text,
          style: TextStyle(
            fontSize: 13,
            fontWeight: isNumeric ? FontWeight.w600 : FontWeight.w400,
            color: context.theme.colors.primary,
          ),
          overflow: TextOverflow.ellipsis,
          maxLines: 2,
        ),
      ),
    );
  }
}
