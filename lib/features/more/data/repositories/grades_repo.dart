import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/more/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesRepository extends CachedRepository<GradeViewData> {
  final String semid;
  final GradesDataSource _dataSource;

  GradesRepository({required this.semid, required GradesDataSource dataSource})
    : _dataSource = dataSource;

  @override
  Future<GradeViewData?> loadCache() async {
    final data = await _dataSource.getGradeView(semid);
    if (data.semesterId.isEmpty) return null;
    return data;
  }

  Future<Map<String, GradeDetailsData>> getGradeDetailsFromStorage() {
    return _dataSource.getGradeDetailsMap(semid);
  }

  @override
  Future<void> saveCache(GradeViewData data) {
    return _dataSource.saveGradeView(data, semid);
  }

  @override
  Future<GradeViewData> fetchRemote() {
    return _dataSource.fetchGradeView(semid);
  }
}

class GradeDetailsRepository extends CachedRepository<GradeDetailsData> {
  GradeDetailsRepository({
    required this.semid,
    required this.courseId,
    required GradesDataSource dataSource,
  }) : _dataSource = dataSource;

  final String semid;
  final String courseId;
  final GradesDataSource _dataSource;

  @override
  Future<GradeDetailsData?> loadCache() async {
    final details = await _dataSource.getGradeDetailsMap(semid);
    return details[courseId];
  }

  @override
  Future<void> saveCache(GradeDetailsData data) {
    return _dataSource.saveGradeDetails(data, semid);
  }

  @override
  Future<GradeDetailsData> fetchRemote() {
    return _dataSource.fetchGradeDetails(semid: semid, courseId: courseId);
  }
}
