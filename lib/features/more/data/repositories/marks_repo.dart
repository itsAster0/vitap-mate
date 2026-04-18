import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/more/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksRepository extends CachedRepository<MarksData> {
  final String semid;
  final MarksDataSource _dataSource;

  MarksRepository({required this.semid, required MarksDataSource dataSource})
    : _dataSource = dataSource;

  @override
  Future<MarksData?> loadCache() async {
    final data = await _dataSource.getMarks(semid);
    if (data.semesterId.isEmpty) return null;
    return data;
  }

  @override
  Future<void> saveCache(MarksData data) {
    return _dataSource.saveMarks(data, semid);
  }

  @override
  Future<MarksData> fetchRemote() {
    return _dataSource.fetchMarks(semid);
  }
}
