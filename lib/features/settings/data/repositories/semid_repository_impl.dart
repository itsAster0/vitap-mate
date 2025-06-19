import 'package:vitapmate/features/settings/data/datasources/local_data_source.dart';
import 'package:vitapmate/features/settings/data/datasources/remote_data_source.dart';
import 'package:vitapmate/features/settings/domine/repositories/semids_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class SemidRepositoryImpl extends SemidRepository {
  final SemesterIdLocalDataSource _localDatasource;
  final SemesterIdRemoteDataSource _remoteDataSource;
  SemidRepositoryImpl(this._localDatasource, this._remoteDataSource);

  @override
  Future<SemesterData> getSemidsFromStorage() async {
    return await _localDatasource.getSemidsFromStorage();
  }

  @override
  Future<SemesterData> updateSemids() async {
    SemesterData data = await _remoteDataSource.fetchSemidsFromremote();
    await _localDatasource.saveSemidsToStorage(data);
    return data;
  }
}
