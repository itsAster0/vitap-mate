import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/more/domine/repositories/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class UpdateExamScheduleUsecase implements Usecase<ExamScheduleData, void> {
  final ExamScheduleRepository _repository;
  UpdateExamScheduleUsecase(this._repository);
  @override
  Future<ExamScheduleData> call({void params}) async {
    await _repository.updateExamSchedule();
    var data = await _repository.getExamScheduleFromStorage();
    return data;
  }
}
