import 'package:vitapmate/features/more/data/datasources/local_data_sources.dart';
import 'package:vitapmate/features/more/data/datasources/remote_data_sources.dart';
import 'package:vitapmate/features/more/domine/repositories/grade_history_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradeHistoryRepoImpl implements GradeHistoryRepository {
  final GradeHistoryRemoteDataSource remoteDataSource;
  final GradeHistoryLocalDataSource localDataSource;

  GradeHistoryRepoImpl({
    required this.remoteDataSource,
    required this.localDataSource,
  });

  @override
  Future<GradeHistoryData> getGradeHistoryFromStorage() async {
    return localDataSource.getGradeHistory();
  }

  @override
  Future<void> saveGradeHistoryToStorage({
    required GradeHistoryData data,
  }) async {
    await localDataSource.saveGradeHistory(data);
  }

  @override
  Future<void> updateGradeHistory() async {
    final data = await remoteDataSource.fetchGradeHistoryFromRemote();
    if (data.records.isEmpty) return;
    await saveGradeHistoryToStorage(data: data);
  }
}
