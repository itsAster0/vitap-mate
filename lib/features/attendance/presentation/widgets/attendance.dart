import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_table.dart';

import 'package:vitapmate/src/api/vtop/types.dart';

class AttendanceCard extends HookConsumerWidget {
  final AttendanceRecord record;

  final int index;
  const AttendanceCard({super.key, required this.record, required this.index});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final expanded = useState(false);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        FTappable(
          onPress: () {
            showFSheet(
              //barrierDismissible: false,
              context: context,
              side: FLayout.btt,
              builder:
                  (context) => AttendanceTable(
                    courseId: record.courseId,
                    courseType: record.courseType,
                    exp: true,
                  ),
            );
          },
          child: Container(
            decoration: BoxDecoration(
              color:
                  record.courseType.endsWith("LA")
                      ? Color(0xFFD4F6FF)
                      : Color(0xFFC6E7FF),
              borderRadius: BorderRadius.circular(8),
              boxShadow: [
                BoxShadow(
                  color: Colors.black12,
                  blurRadius: 10,
                  offset: Offset(0, 4),
                ),
              ],
            ),
            child: Padding(
              padding: const EdgeInsets.all(8.0),

              child: Column(
                spacing: 8,
                children: [
                  Row(
                    spacing: 2,
                    children: [
                      Icon(
                        record.courseType.endsWith("LA")
                            ? FIcons.flaskConical
                            : FIcons.libraryBig,
                      ),
                      Expanded(
                        child: SizedBox(
                          width: MediaQuery.of(context).size.width * 0.8,
                          child: Text(
                            formateName(record.courseName).$2,
                            maxLines: 1,
                            overflow: TextOverflow.fade,
                            softWrap: false,
                            style: TextStyle(fontWeight: FontWeight.w500),
                          ),
                        ),
                      ),
                      SizedBox(width: 8),
                      SizedBox(
                        child: Text(
                          "•${formateName(record.courseName).$1}",
                          style: TextStyle(fontSize: 12),
                        ),
                      ),
                    ],
                  ),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                    children: [
                      Expanded(
                        child: Column(
                          spacing: 2,
                          crossAxisAlignment: CrossAxisAlignment.center,
                          children: [
                            Text(
                              record.attendancePercentage,
                              style: TextStyle(
                                fontSize: 20,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                            Text("Total", maxLines: 2),
                          ],
                        ),
                      ),
                      if (record.attendancePercentage != "-")
                        Expanded(
                          child: Column(
                            spacing: 2,
                            children: [
                              Text(
                                record.attendenceFatCat,
                                style: TextStyle(
                                  fontSize: 20,
                                  fontWeight: FontWeight.bold,
                                  //color: double.parse(record.attendancePercentage.replaceAll("%", ""))>=75?	Color.fromARGB(255, 23, 189, 5):	Color.fromARGB(255, 203, 67, 67),
                                ),
                              ),
                              Text("Btw. Exams", maxLines: 2),
                            ],
                          ),
                        ),

                      Expanded(
                        child: Center(
                          child: Column(
                            spacing: 4,
                            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text("Present ${record.classesAttended} "),
                              Text("Total ${record.totalClasses} "),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
        const SizedBox(height: 6),
      ],
    );
  }
}

(String, String) formateName(String name) {
  var splitName = name.split("-");
  return (splitName[0], splitName[1]);
}
