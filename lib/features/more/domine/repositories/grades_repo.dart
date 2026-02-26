import 'package:vitapmate/src/api/vtop/types.dart';

abstract class GradesRepository {
  Future<GradeViewData> getGradeViewFromStorage();
  Future<Map<String, GradeDetailsData>> getGradeDetailsFromStorage();
  Future<void> saveGradeViewToStorage({required GradeViewData data});
  Future<void> saveGradeDetailsToStorage({required GradeDetailsData data});
  Future<void> updateGradeView();
  Future<GradeDetailsData> fetchGradeDetailsRemote({required String courseId});
}
