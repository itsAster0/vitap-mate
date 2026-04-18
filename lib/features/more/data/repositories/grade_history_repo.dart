import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/more/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradeHistoryRepository extends CachedRepository<GradeHistoryData> {
  final GradeHistoryDataSource _dataSource;

  GradeHistoryRepository({required GradeHistoryDataSource dataSource})
    : _dataSource = dataSource;

  @override
  Future<GradeHistoryData?> loadCache() async {
    final data = await _dataSource.getGradeHistory();
    final isEmptyCache =
        data.records.isEmpty &&
        data.student.regNo.isEmpty &&
        data.updateTime == BigInt.zero;
    if (isEmptyCache) return null;
    return data;
  }

  @override
  Future<void> saveCache(GradeHistoryData data) {
    return _dataSource.saveGradeHistory(data);
  }

  @override
  Future<GradeHistoryData> fetchRemote() {
    return _dataSource.fetchGradeHistory();
  }
}
