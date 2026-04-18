import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/more/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamScheduleRepository extends CachedRepository<ExamScheduleData> {
  final String semid;
  final ExamScheduleDataSource _dataSource;

  ExamScheduleRepository({
    required this.semid,
    required ExamScheduleDataSource dataSource,
  }) : _dataSource = dataSource;

  @override
  Future<ExamScheduleData?> loadCache() async {
    final data = await _dataSource.getExamSchedule(semid);
    if (data.semesterId.isEmpty) return null;
    return data;
  }

  @override
  Future<void> saveCache(ExamScheduleData data) {
    return _dataSource.saveExamSchedule(data, semid);
  }

  @override
  Future<ExamScheduleData> fetchRemote() {
    return _dataSource.fetchExamSchedule(semid);
  }
}
