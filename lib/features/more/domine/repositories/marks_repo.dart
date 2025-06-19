import 'package:vitapmate/src/api/vtop/types.dart';

abstract class MarksRepository {
  Future<MarksData> getMarksFromStorage();
  Future<void> saveMarksToStorage({required MarksData data});
  Future<void> updateMarks();
}
