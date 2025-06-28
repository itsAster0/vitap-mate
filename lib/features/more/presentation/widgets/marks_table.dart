import 'package:flutter/material.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksTable extends StatelessWidget {
  final List<MarksRecordEach> marks;
  const MarksTable({super.key, required this.marks});

  @override
  Widget build(BuildContext context) {
    return Container(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.of(context).size.height / 2.5,
      ),
      decoration: BoxDecoration(
        color: MarksColors.tableBackground,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: MarksColors.tableBorder),
        boxShadow: [
          BoxShadow(
            color: MarksColors.cardShadow,
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
            scrollDirection: Axis.vertical,
            child: DataTable(
              decoration: const BoxDecoration(),
              dividerThickness: 1,
              headingRowColor: WidgetStateProperty.all(
                MarksColors.tableHeaderBackground,
              ),
              headingRowHeight: 56,
              dataRowMinHeight: 48,
              dataRowMaxHeight: 64,
              columnSpacing: 16,
              horizontalMargin: 16,
              border: TableBorder.all(
                color: MarksColors.tableBorder,
                borderRadius: const BorderRadius.all(Radius.circular(4)),
              ),
              columns: [
                _buildDataColumn(""),
                _buildDataColumn("Title"),
                _buildDataColumn("Scored Mark"),
                _buildDataColumn("Max Marks"),
                _buildDataColumn("Weightage Mark"),
                _buildDataColumn("Weightage%"),
                _buildDataColumn("Status"),
                _buildDataColumn("Remark"),
              ],
              rows:
                  marks.asMap().entries.map<DataRow>((entry) {
                    final index = entry.key;
                    final mark = entry.value;
                    final isEven = index % 2 == 0;

                    return DataRow(
                      color: WidgetStateProperty.all(
                        isEven
                            ? Colors.transparent
                            : MarksColors.tableRowAlternate,
                      ),
                      cells: [
                        _buildDataCell(mark.serial, isNumeric: true),
                        _buildDataCell(mark.markstitle),
                        _buildDataCell(mark.scoredmark, isNumeric: true),
                        _buildDataCell(mark.maxmarks, isNumeric: true),
                        _buildDataCell(mark.weightagemark, isNumeric: true),
                        _buildDataCell(mark.weightage),
                        DataCell(_buildStatusBadge(mark.status)),
                        _buildDataCell(mark.remark),
                      ],
                    );
                  }).toList(),
            ),
          ),
        ),
      ),
    );
  }

  DataColumn _buildDataColumn(String label) {
    return DataColumn(
      label: Text(
        label,
        style: const TextStyle(
          fontWeight: FontWeight.w600,
          fontSize: 14,
          color: MarksColors.primaryText,
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
          color: MarksColors.secondaryText,
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

    if (statusLower.contains('pass') || statusLower.contains('complete')) {
      backgroundColor = MarksColors.passedBackground;
      textColor = MarksColors.passedText;
      borderColor = MarksColors.passedBorder;
      icon = Icons.check_circle_outline;
    } else if (statusLower.contains('fail')) {
      backgroundColor = MarksColors.failedBackground;
      textColor = MarksColors.failedText;
      borderColor = MarksColors.failedBorder;
      icon = Icons.cancel_outlined;
    } else {
      backgroundColor = MarksColors.pendingBackground;
      textColor = MarksColors.pendingText;
      borderColor = MarksColors.pendingBorder;
      icon = Icons.schedule_outlined;
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
}
