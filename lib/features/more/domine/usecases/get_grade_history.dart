import 'package:vitapmate/features/more/domine/repositories/grade_history_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GetGradeHistoryUsecase {
  final GradeHistoryRepository _repo;
  GetGradeHistoryUsecase(this._repo);

  Future<GradeHistoryData> call() async {
    return _repo.getGradeHistoryFromStorage();
  }
}
