import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/timetable/domine/repositories/timetable_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GetTimetableUsecase implements Usecase<TimetableData, void> {
  final TimetableRepository _timetableRepository;
  GetTimetableUsecase(this._timetableRepository);
  @override
  Future<TimetableData> call({void params}) async {
    var timetable = await _timetableRepository.getTimetableFromStorage();
    return timetable;
  }
}
