import 'package:flutter/material.dart';

import 'package:vitapmate/src/api/vtop/types.dart';

class MarksTable extends StatelessWidget {
  final List<MarksRecordEach> marks;
  const MarksTable({super.key, required this.marks});

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.of(context).size.height / 2.5,
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: SingleChildScrollView(
          scrollDirection: Axis.vertical,
          child: Column(
            children: [
              DataTable(
                decoration: BoxDecoration(),
                dividerThickness: 0,
                border: TableBorder.all(
                  borderRadius: BorderRadius.all(Radius.circular(4)),
                ),

                columns: [
                  DataColumn(label: Text("")),
                  DataColumn(label: Text("Title")),
                  DataColumn(label: Text("Scored.Mark")),
                  DataColumn(label: Text("Max.Marks")),
                  DataColumn(label: Text("Weightage.Mark")),
                  DataColumn(label: Text("Weightage%")),
                  DataColumn(label: Text("Status")),
                  DataColumn(label: Text("Remark")),
                ],
                rows: [
                  for (var i in marks)
                    DataRow(
                      cells: [
                        DataCell(Text(i.serial)),
                        DataCell(Text(i.markstitle)),
                        DataCell(Text(i.scoredmark)),
                        DataCell(Text(i.maxmarks)),
                        DataCell(Text(i.weightagemark)),
                        DataCell(Text(i.weightage)),
                        DataCell(Text(i.status)),
                        DataCell(Text(i.remark)),
                      ],
                    ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
