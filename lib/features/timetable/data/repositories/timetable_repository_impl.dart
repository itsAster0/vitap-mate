import 'package:vitapmate/features/timetable/data/datasources/local_data_source.dart';
import 'package:vitapmate/features/timetable/data/datasources/remote_data_source.dart';
import 'package:vitapmate/features/timetable/domine/repositories/timetable_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetableRepositoryImpl extends TimetableRepository {
  final String semid;
  final TimetableRemoteDataSource remoteDataSource;
  final TimetableLocalDataSource localDataSource;

  TimetableRepositoryImpl({
    required this.semid,
    required this.remoteDataSource,
    required this.localDataSource,
  });
  @override
  Future<TimetableData> getTimetableFromStorage() async {
    var timetable = await localDataSource.getTimetable(semid);
    return timetable;
  }

  @override
  Future<void> saveTimetableToStorage({
    required TimetableData timetable,
  }) async {
    await localDataSource.saveTimetable(timetable, semid);
  }

  @override
  Future<void> updateTimetable() async {
    var timetable = await remoteDataSource.fetchTimetableFromRemote(semid);
    if (timetable.slots.isEmpty) return;
    saveTimetableToStorage(timetable: timetable);
  }
}
