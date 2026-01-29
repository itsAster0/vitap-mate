import 'package:vitapmate/features/more/data/datasources/local_data_sources.dart';
import 'package:vitapmate/features/more/data/datasources/remote_data_sources.dart';
import 'package:vitapmate/features/more/domine/repositories/marks_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksRepoImpl implements MarksRepository {
  final String semid;
  final MarksRemoteDataSource remoteDataSource;
  final MarksLocalDataSource localDataSource;

  MarksRepoImpl({
    required this.semid,
    required this.remoteDataSource,
    required this.localDataSource,
  });
  @override
  Future<MarksData> getMarksFromStorage() async {
    var data = await localDataSource.getMarks(semid);
    return data;
  }

  @override
  Future<void> saveMarksToStorage({required MarksData data}) async {
    await localDataSource.saveMarks(data, semid);
  }

  @override
  Future<void> updateMarks() async {
    var data = await remoteDataSource.fetchMarksFromRemote(semid);
    if (data.records.isEmpty) return;
    saveMarksToStorage(data: data);
  }
}
