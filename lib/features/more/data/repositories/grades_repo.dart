import 'package:vitapmate/features/more/data/datasources/local_data_sources.dart';
import 'package:vitapmate/features/more/data/datasources/remote_data_sources.dart';
import 'package:vitapmate/features/more/domine/repositories/grades_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesRepoImpl implements GradesRepository {
  final String semid;
  final GradesRemoteDataSource remoteDataSource;
  final GradesLocalDataSource localDataSource;

  GradesRepoImpl({
    required this.semid,
    required this.remoteDataSource,
    required this.localDataSource,
  });

  @override
  Future<GradeViewData> getGradeViewFromStorage() async {
    return localDataSource.getGradeView(semid);
  }

  @override
  Future<Map<String, GradeDetailsData>> getGradeDetailsFromStorage() async {
    return localDataSource.getGradeDetailsMap(semid);
  }

  @override
  Future<void> saveGradeViewToStorage({required GradeViewData data}) async {
    await localDataSource.saveGradeView(data, semid);
  }

  @override
  Future<void> saveGradeDetailsToStorage({
    required GradeDetailsData data,
  }) async {
    await localDataSource.saveGradeDetails(data, semid);
  }

  @override
  Future<void> updateGradeView() async {
    final data = await remoteDataSource.fetchGradeViewFromRemote(semid);
    if (data.courses.isEmpty) return;
    await saveGradeViewToStorage(data: data);
  }

  @override
  Future<GradeDetailsData> fetchGradeDetailsRemote({
    required String courseId,
  }) async {
    final data = await remoteDataSource.fetchGradeDetailsFromRemote(
      semid: semid,
      courseId: courseId,
    );
    await saveGradeDetailsToStorage(data: data);
    return data;
  }
}
