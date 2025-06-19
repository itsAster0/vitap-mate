import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/settings/domine/repositories/semids_repo.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GetSemidsUsecase implements Usecase<SemesterData, void> {
  final SemidRepository _semidRepository;
  GetSemidsUsecase(this._semidRepository);
  @override
  Future<SemesterData> call({void params}) async {
    return await _semidRepository.getSemidsFromStorage();
  }
}
