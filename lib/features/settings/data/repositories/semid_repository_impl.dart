import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/settings/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class SemidRepository extends CachedRepository<SemesterData> {
  final SemesterIdDataSource _dataSource;

  SemidRepository(this._dataSource);

  @override
  Future<SemesterData?> loadCache() async {
    final data = await _dataSource.getSemidsFromStorage();
    if (data.semesters.isEmpty) return null;
    return data;
  }

  @override
  Future<void> saveCache(SemesterData data) {
    return _dataSource.saveSemidsToStorage(data);
  }

  @override
  Future<SemesterData> fetchRemote() {
    return _dataSource.fetchSemids();
  }
}
