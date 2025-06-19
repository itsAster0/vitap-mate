import 'package:flutter/material.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamScheduleCard extends StatelessWidget {
  final PerExamScheduleRecord record;
  const ExamScheduleCard({super.key, required this.record});

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Center(
          child: Text(
            record.examType,
            style: TextStyle(fontWeight: FontWeight.bold),
          ),
        ),
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: DataTable(
            columns: [
              DataColumn(label: Text("")),
              DataColumn(label: Text("Course")),
              DataColumn(label: Text("Date")),
              DataColumn(label: Text("Venue")),
              DataColumn(label: Text("Seat")),
              DataColumn(label: Text("Seat.No")),
              DataColumn(label: Text("Time")),
              DataColumn(label: Text("Report.Time")),
              DataColumn(label: Text("Session")),
              DataColumn(label: Text("Slot")),
              DataColumn(label: Text("CourseType")),
              DataColumn(label: Text("CourseName")),
              DataColumn(label: Text("Class.ID")),
            ],
            rows: [
              for (var i in record.records)
                DataRow(
                  cells: [
                    DataCell(Text(i.serial)),
                    DataCell(Text(i.courseCode)),
                    DataCell(Text(i.examDate)),
                    DataCell(Text(i.venue)),
                    DataCell(Text(i.seatLocation)),
                    DataCell(Text(i.seatNo)),
                    DataCell(Text(i.examTime)),
                    DataCell(Text(i.reportingTime)),
                    DataCell(Text(i.examSession)),
                    DataCell(Text(i.slot)),
                    DataCell(Text(i.courseType)),
                    DataCell(Text(i.courseName)),
                    DataCell(Text(i.courseId)),
                  ],
                ),
            ],
          ),
        ),
      ],
    );
  }
}
