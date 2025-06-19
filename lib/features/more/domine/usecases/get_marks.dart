import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/more/domine/repositories/marks_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GetMarksUsecase implements Usecase<MarksData, void> {
  final MarksRepository _repository;
  GetMarksUsecase(this._repository);
  @override
  Future<MarksData> call({void params}) async {
    var data = await _repository.getMarksFromStorage();
    return data;
  }
}
