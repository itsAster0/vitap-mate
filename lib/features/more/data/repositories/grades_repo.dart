import 'package:vitapmate/features/more/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesRepository {
  final String semid;
  final GradesDataSource _dataSource;

  GradesRepository({required this.semid, required GradesDataSource dataSource})
    : _dataSource = dataSource;

  Future<GradeViewData> getGradeViewFromStorage() {
    return _dataSource.getGradeView(semid);
  }

  Future<Map<String, GradeDetailsData>> getGradeDetailsFromStorage() {
    return _dataSource.getGradeDetailsMap(semid);
  }

  Future<void> saveGradeViewToStorage({required GradeViewData data}) {
    return _dataSource.saveGradeView(data, semid);
  }

  Future<void> saveGradeDetailsToStorage({required GradeDetailsData data}) {
    return _dataSource.saveGradeDetails(data, semid);
  }

  Future<void> updateGradeView() async {
    final data = await _dataSource.fetchGradeView(semid);
    if (data.courses.isEmpty) return;
    await saveGradeViewToStorage(data: data);
  }

  Future<GradeDetailsData> fetchGradeDetailsRemote({
    required String courseId,
  }) async {
    final data = await _dataSource.fetchGradeDetails(
      semid: semid,
      courseId: courseId,
    );
    await saveGradeDetailsToStorage(data: data);
    return data;
  }
}
