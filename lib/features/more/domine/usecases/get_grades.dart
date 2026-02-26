import 'package:vitapmate/features/more/domine/repositories/grades_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GetGradeViewUsecase {
  final GradesRepository _repo;
  GetGradeViewUsecase(this._repo);

  Future<GradeViewData> call() async {
    return _repo.getGradeViewFromStorage();
  }
}

class GetGradeDetailsMapUsecase {
  final GradesRepository _repo;
  GetGradeDetailsMapUsecase(this._repo);

  Future<Map<String, GradeDetailsData>> call() async {
    return _repo.getGradeDetailsFromStorage();
  }
}
