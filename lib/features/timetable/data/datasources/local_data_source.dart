import 'package:drift/drift.dart';
import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/timetable/data/models/timetable_model.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetableLocalDataSource {
  final AppDatabase _db;
  final AsyncQueue _globalAsyncQueue;
  TimetableLocalDataSource(this._db, this._globalAsyncQueue);

  Future<TimetableData> getTimetable(String semid) async {
    final allRows = await _globalAsyncQueue.run(
      "fromStorage_timetable_$semid",
      () =>
          (_db.select(_db.timetableTable)
            ..where((tbl) => tbl.semId.equals(semid))).get(),
    );
    if (allRows.isEmpty) {
      return TimetableData(
        slots: [],
        semesterId: "",
        updateTime: BigInt.from(0),
      );
    }
    return TimetableModel.toEntityFromLocal(allRows);
  }

  Future<void> saveTimetable(TimetableData timetable, String semid) async {
    await _globalAsyncQueue.run(
      "toStroage_timetable_$semid",
      () => (_db.batch((batch) {
        batch.deleteWhere(_db.timetableTable, (tbl) => tbl.semId.equals(semid));
        batch.insertAll(_db.timetableTable, [
          for (var i in timetable.slots)
            TimetableTableCompanion.insert(
              serial: int.parse(i.serial),
              day: i.day,
              slot: i.slot,
              courseCode: i.courseCode,
              courseType: i.courseType,
              courseName: i.name,
              roomNo: i.roomNo,
              block: i.block,
              startTime: i.startTime,
              endTime: i.endTime,
              semId: timetable.semesterId,
              time: timetable.updateTime.toInt(),
              faculty: Value(i.faculty),
              isLab: Value(i.isLab),
            ),
        ]);
      })),
    );
  }
}
