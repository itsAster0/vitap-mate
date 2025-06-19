import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/more/domine/repositories/marks_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class UpdateMarksUsecase implements Usecase<MarksData, void> {
  final MarksRepository _repository;
  UpdateMarksUsecase(this._repository);
  @override
  Future<MarksData> call({void params}) async {
    await _repository.updateMarks();
    var data = await _repository.getMarksFromStorage();
    return data;
  }
}
