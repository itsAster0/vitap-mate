import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/timetable/domine/repositories/timetable_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class UpdateTimetableUsecase implements Usecase<TimetableData, void> {
  final TimetableRepository _timetableRepository;
  UpdateTimetableUsecase(this._timetableRepository);
  @override
  Future<TimetableData> call({void params}) async {
    await _timetableRepository.updateTimetable();
    var data = await _timetableRepository.getTimetableFromStorage();
    return data;
  }
}
