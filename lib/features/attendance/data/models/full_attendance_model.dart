import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class FullAttendanceModel {
  static FullAttendanceData toEntityFromLocal(
    List<FullAttendanceTableData> atten,
    String courseType,
    String courseId,
  ) {
    List<FullAttendanceRecord> sub = [];
    for (var i in atten) {
      var temp = FullAttendanceRecord(
        serial: "${i.serial}",
        date: i.date,
        slot: i.slot,
        dayTime: i.dayTime,
        status: i.status,
        remark: i.remark,
      );
      sub.add(temp);
    }

    return FullAttendanceData(
      records: sub,
      semesterId: atten.isEmpty ? "" : atten[0].semId,
      courseId: courseId,
      courseType: courseType,
      updateTime: atten.isEmpty ? BigInt.from(0) : BigInt.from(atten[0].time),
    );
  }
}
