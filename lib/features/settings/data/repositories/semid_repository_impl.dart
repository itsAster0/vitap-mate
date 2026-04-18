import 'package:vitapmate/features/settings/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class SemidRepository {
  final SemesterIdDataSource _dataSource;

  SemidRepository(this._dataSource);

  Future<SemesterData> getSemidsFromStorage() {
    return _dataSource.getSemidsFromStorage();
  }

  Future<SemesterData> updateSemids() async {
    final data = await _dataSource.fetchSemids();
    await _dataSource.saveSemidsToStorage(data);
    return data;
  }
}
