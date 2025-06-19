import 'dart:convert';

import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksModel {
  static MarksData toEntityFromLocal(List<MarksTableData> data) {
    Map<String, MarksRecord> tempMap = {};
    for (var i in data) {
      String sig = "${i.courseCode}_${i.courseType}";
      MarksRecordEach marks = MarksRecordEach.fromJson(jsonDecode(i.marks));
      if (tempMap[sig] == null) {
        tempMap[sig] = MarksRecord(
          serial: "${i.serial}",
          coursecode: i.courseCode,
          coursetitle: i.courseTitle,
          coursetype: i.courseType,
          faculity: i.faculty,
          slot: i.slot,
          marks: [marks],
        );
      } else {
        tempMap[sig] = tempMap[sig]!.copyWith(
          marks: [...tempMap[sig]!.marks, marks],
        );
      }
    }

    return MarksData(
      records: tempMap.values.toList(),
      semesterId: data[0].semId,
      updateTime: BigInt.from(data[0].time),
    );
  }
}
