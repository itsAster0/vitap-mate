import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AttendanceModel {
  static AttendanceData toEntityFromLocal(List<AttendanceTableData> atten) {
    List<AttendanceRecord> sub = [];
    for (var i in atten) {
      var temp = AttendanceRecord(
        serial: "${i.serial}",
        category: i.category,
        courseName: i.courseName,
        courseCode: i.courseCode,
        courseType: i.courseType,
        facultyDetail: i.facultyDetail,
        classesAttended: i.classesAttended,
        totalClasses: i.totalClasses,
        attendancePercentage: i.attendancePercentage,
        attendenceFatCat: i.attendenceFatCat,
        debarStatus: i.debarStatus,
        courseId: i.courseId,
      );
      sub.add(temp);
    }
    return AttendanceData(
      records: sub,
      semesterId: atten.isEmpty ? "" : atten[0].semId,
      updateTime: atten.isEmpty ? BigInt.from(0) : BigInt.from(atten[0].time),
    );
  }
}
