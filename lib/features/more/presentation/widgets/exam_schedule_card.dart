import 'package:flutter/material.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamScheduleCard extends StatelessWidget {
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

  String _getExamStatus() {
    final upcomingCount = _getUpcomingExamsCount();
    final totalCount = record.records.length;

    if (upcomingCount == 0) return "Completed";
    if (upcomingCount == totalCount) return "Upcoming";
    return "In Progress";
  }

  Color _getStatusColor() {
    final status = _getExamStatus();
    switch (status) {
      case "Completed":
        return ExamColors.completedText;
      case "Upcoming":
        return ExamColors.upcomingText;
      default:
        return ExamColors.todayText;
    }
  }

  @override
  Widget build(BuildContext context) {
    final upcomingCount = _getUpcomingExamsCount();
    final status = _getExamStatus();

    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: Container(
        decoration: BoxDecoration(
          gradient: const LinearGradient(
            colors: [
              ExamColors.examCardBackground,
              ExamColors.examCardSecondary,
            ],
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
          ),
          borderRadius: BorderRadius.circular(16),
          boxShadow: [
            BoxShadow(
              color: ExamColors.cardShadow,
              blurRadius: 12,
              offset: const Offset(0, 4),
            ),
          ],
          border: Border.all(
            color: Colors.white.withValues(alpha: 0.8),
            width: 1,
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.white.withValues(alpha: 0.9),
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
                          color: ExamColors.todayBackground,
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
                          style: const TextStyle(
                            fontWeight: FontWeight.w700,
                            fontSize: 18,
                            color: ExamColors.primaryText,
                          ),
                        ),
                      ),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 6,
                        ),
                        decoration: BoxDecoration(
                          color: _getStatusColor().withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(20),
                          border: Border.all(
                            color: _getStatusColor().withValues(alpha: 0.3),
                            width: 1,
                          ),
                        ),
                        child: Text(
                          status,
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                            color: _getStatusColor(),
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
                        color: ExamColors.secondaryText,
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
              decoration: const BoxDecoration(
                color: ExamColors.tableBackground,
              ),
              child: ClipRRect(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: DataTable(
                    decoration: const BoxDecoration(),
                    dividerThickness: 1,
                    headingRowColor: WidgetStateProperty.all(
                      ExamColors.tableHeaderBackground,
                    ),
                    headingRowHeight: 56,
                    dataRowMinHeight: 48,
                    dataRowMaxHeight: 64,
                    columnSpacing: 16,
                    horizontalMargin: 16,
                    columns: [
                      _buildDataColumn(""),
                      _buildDataColumn("Course"),
                      _buildDataColumn("Date"),
                      _buildDataColumn("Venue"),
                      _buildDataColumn("Seat"),
                      _buildDataColumn("Seat No."),
                      _buildDataColumn("Time"),
                      _buildDataColumn("Report Time"),
                      _buildDataColumn("Session"),
                      _buildDataColumn("Slot"),
                      _buildDataColumn("Course Type"),
                      _buildDataColumn("Course Name"),
                      _buildDataColumn("Class ID"),
                    ],
                    rows:
                        record.records.asMap().entries.map<DataRow>((entry) {
                          final index = entry.key;
                          final exam = entry.value;
                          final isEven = index % 2 == 0;
                          final isUpcoming = _isUpcomingExam(exam.examDate);

                          return DataRow(
                            color: WidgetStateProperty.all(
                              isUpcoming
                                  ? ExamColors.upcomingBackground.withValues(
                                    alpha: 0.3,
                                  )
                                  : isEven
                                  ? Colors.transparent
                                  : ExamColors.tableRowAlternate,
                            ),
                            cells: [
                              _buildDataCell(exam.serial, isNumeric: true),
                              _buildDataCell(exam.courseCode),
                              _buildDataCell(
                                exam.examDate,
                                isDate: true,
                                isUpcoming: isUpcoming,
                              ),
                              _buildDataCell(exam.venue),
                              _buildDataCell(exam.seatLocation),
                              _buildDataCell(exam.seatNo),
                              _buildDataCell(exam.examTime),
                              _buildDataCell(exam.reportingTime),
                              _buildDataCell(exam.examSession),
                              _buildDataCell(exam.slot),
                              _buildDataCell(exam.courseType),
                              _buildDataCell(exam.courseName),
                              _buildDataCell(exam.courseId),
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

  DataColumn _buildDataColumn(String label) {
    return DataColumn(
      label: Text(
        label,
        style: const TextStyle(
          fontWeight: FontWeight.w600,
          fontSize: 14,
          color: ExamColors.primaryText,
        ),
      ),
    );
  }

  DataCell _buildDataCell(
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
            color:
                isUpcoming && isDate
                    ? ExamColors.upcomingText
                    : ExamColors.secondaryText,
          ),
          overflow: TextOverflow.ellipsis,
          maxLines: 2,
        ),
      ),
    );
  }
}
