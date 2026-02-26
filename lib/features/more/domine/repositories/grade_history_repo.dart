import 'package:vitapmate/src/api/vtop/types.dart';

abstract class GradeHistoryRepository {
  Future<GradeHistoryData> getGradeHistoryFromStorage();
  Future<void> saveGradeHistoryToStorage({required GradeHistoryData data});
  Future<void> updateGradeHistory();
}
