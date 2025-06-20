import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/features/attendance/presentation/providers/full_attendance_provider.dart';

class AttendanceTable extends HookConsumerWidget {
  final String courseId;
  final String courseType;

  final bool exp;
  const AttendanceTable({
    super.key,
    required this.courseId,
    required this.courseType,
    required this.exp,
  });
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var dataAsync = ref.watch(FullAttendanceProvider(courseType, courseId));
    return Container(
      height: double.infinity,
      width: double.infinity,
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        border:
            FLayout.btt.vertical
                ? Border.symmetric(
                  horizontal: BorderSide(color: context.theme.colors.border),
                )
                : Border.symmetric(
                  vertical: BorderSide(color: context.theme.colors.border),
                ),
      ),
      child: dataAsync.when(
        data: (data) {
          if (data.records.isEmpty) {
            return Center(child: Text("No Data to show yet"));
          }
          return Padding(
            padding: const EdgeInsets.all(8.0),
            child: FFocusedOutline(
              focused: false,
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: SingleChildScrollView(
                  scrollDirection: Axis.vertical,
                  child: Padding(
                    padding: const EdgeInsets.all(0.0),
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
                            DataColumn(label: Text("Date")),
                            DataColumn(label: Text("Status")),
                            DataColumn(label: Text("Time")),
                            DataColumn(label: Text("Slot")),
                            DataColumn(label: Text("Remark")),
                          ],
                          rows: [
                            for (var i in data.records)
                              DataRow(
                                cells: [
                                  DataCell(Text(i.serial)),
                                  DataCell(Text(i.date)),
                                  if (i.status == "Absent" ||
                                      i.status == "Present")
                                    DataCell(
                                      FBadge(
                                        style:
                                            i.status == "Absent"
                                                ? FBadgeStyle.destructive
                                                : FBadgeStyle.secondary,
                                        child: Text(i.status),
                                      ),
                                    ),
                                  if (i.status != "Absent" &&
                                      i.status != "Present")
                                    DataCell(
                                      FBadge(
                                        style: FBadgeStyle.primary,
                                        child: Text(i.status),
                                      ),
                                    ),
                                  DataCell(Text(i.dayTime)),
                                  DataCell(Text(i.slot)),
                                  DataCell(Text(i.remark)),
                                ],
                              ),
                          ],
                        ),

                        Center(
                          child: Padding(
                            padding: const EdgeInsets.symmetric(vertical: 12.0),
                            child: Text(
                              "Data updated on ${formatUnixTimestamp(data.updateTime.toInt())}",
                              style: TextStyle(
                                fontSize: 14,
                                color: Colors.grey[600],
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          );
        },
        error: (e, se) {
          String msg = commonErrorMessage(e);
          return Center(child: Text(msg));
        },
        loading:
            () => Center(
              child: SizedBox(
                width: 50,
                height: 50,
                child: CircularProgressIndicator(color: Colors.black),
              ),
            ),
      ),
    );
  }
}
