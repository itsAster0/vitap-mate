import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetableModel {
  static toEntityFromLocal(List<TimetableTableData> rtimetable) {
    List<TimetableSlot> sub = [];
    for (var i in rtimetable) {
      var temp = TimetableSlot(
        block: i.block,
        courseCode: i.courseCode,
        courseType: i.courseType,
        day: i.day,
        endTime: i.endTime,
        name: i.courseName,
        roomNo: i.roomNo,
        serial: "${i.serial}",
        slot: i.slot,
        startTime: i.startTime,
      );
      sub.add(temp);
    }
    return TimetableData(
      slots: sub,
      semesterId: rtimetable[0].semId,
      updateTime: BigInt.from(rtimetable[0].time),
    );
  }
}
