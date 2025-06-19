import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamsScheduleModel {
  static ExamScheduleData toEntityFromLocal(List<ExamScheduleTableData> data) {
    Map<String, List<ExamScheduleRecord>> tempMap = {};
    for (var i in data) {
      String sig = i.examType;
      tempMap[sig] = [
        ...?tempMap[sig],
        ExamScheduleRecord(
          serial: "${i.serial}",
          slot: i.slot,
          courseName: i.courseName,
          courseCode: i.courseCode,
          courseType: i.courseType,
          courseId: i.courseId,
          examDate: i.examDate,
          examSession: i.examSession,
          reportingTime: i.reportingTime,
          examTime: i.examDate,
          venue: i.venue,
          seatLocation: i.seatLocation,
          seatNo: i.seatNo,
        ),
      ];
    }
    List<PerExamScheduleRecord> perexam = [];
    for (final i in tempMap.entries) {
      perexam.add(PerExamScheduleRecord(records: i.value, examType: i.key));
    }

    return ExamScheduleData(
      exams: perexam,
      semesterId: data[0].semId,
      updateTime: BigInt.from(data[0].time),
    );
  }
}
