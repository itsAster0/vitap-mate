import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class SemidsModel {
  static fromLocalToEntity(List<SemesterTableData> data) {
    List<SemesterInfo> semids = [];
    for (var i in data) {
      semids.add(SemesterInfo(name: i.semName, id: i.semid));
    }
    if (data.isEmpty) {
      return SemesterData(semesters: [], updateTime: BigInt.from(0));
    }
    return SemesterData(
      semesters: semids,
      updateTime: BigInt.from(data[0].time),
    );
  }
}
