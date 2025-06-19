import 'package:vitapmate/src/api/vtop/types.dart';

abstract class ExamScheduleRepository {
  Future<ExamScheduleData> getExamScheduleFromStorage();
  Future<void> saveExamScheduleToStorage({required ExamScheduleData data});
  Future<void> updateExamSchedule();
}
