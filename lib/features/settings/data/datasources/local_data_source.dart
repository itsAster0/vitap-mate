import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/settings/data/models/semsester_model.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class SemesterIdLocalDataSource {
  final AppDatabase _appDatabase;
  final GlobalAsyncQueue _globalAsyncQueue;
  SemesterIdLocalDataSource(this._globalAsyncQueue, this._appDatabase);

  Future<SemesterData> getSemidsFromStorage() async {
    List<SemesterTableData> data = await _globalAsyncQueue.run(
      "get_semids_storage",
      () => _appDatabase.select(_appDatabase.semesterTable).get(),
    );
    return SemidsModel.fromLocalToEntity(data);
  }

  Future<void> saveSemidsToStorage(SemesterData semid) async {
    await _globalAsyncQueue.run(
      "toStriage_timetable_$semid",
      () => (_appDatabase.batch((batch) {
        batch.deleteAll(_appDatabase.semesterTable);
        batch.insertAll(_appDatabase.semesterTable, [
          for (var i in semid.semesters)
            SemesterTableCompanion.insert(
              semName: i.name,
              semid: i.id,
              time: semid.updateTime.toInt(),
            ),
        ]);
      })),
    );
  }
}
