import 'package:vitapmate/features/more/domine/repositories/grade_history_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class UpdateGradeHistoryUsecase {
  final GradeHistoryRepository _repo;
  UpdateGradeHistoryUsecase(this._repo);

  Future<GradeHistoryData> call() async {
    await _repo.updateGradeHistory();
    return _repo.getGradeHistoryFromStorage();
  }
}
