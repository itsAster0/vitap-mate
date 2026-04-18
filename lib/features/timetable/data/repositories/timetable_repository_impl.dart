import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/timetable/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetableRepository extends CachedRepository<TimetableData> {
  final String semid;
  final TimetableDataSource _dataSource;

  TimetableRepository({
    required this.semid,
    required TimetableDataSource dataSource,
  }) : _dataSource = dataSource;

  @override
  Future<TimetableData?> loadCache() async {
    final timetable = await _dataSource.getTimetable(semid);
    if (timetable.semesterId.isEmpty) return null;
    return timetable;
  }

  @override
  Future<void> saveCache(TimetableData data) async {
    await _dataSource.saveTimetable(data, semid);
  }

  @override
  Future<TimetableData> fetchRemote() async {
    return _dataSource.fetchTimetable(semid);
  }
}
