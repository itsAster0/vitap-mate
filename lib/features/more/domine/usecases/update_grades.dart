import 'package:vitapmate/features/more/domine/repositories/grades_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class UpdateGradeViewUsecase {
  final GradesRepository _repo;
  UpdateGradeViewUsecase(this._repo);

  Future<void> call() async {
    await _repo.updateGradeView();
  }
}

class FetchGradeDetailsUsecase {
  final GradesRepository _repo;
  FetchGradeDetailsUsecase(this._repo);

  Future<GradeDetailsData> call(String courseId) async {
    return _repo.fetchGradeDetailsRemote(courseId: courseId);
  }
}
