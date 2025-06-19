import 'package:vitapmate/src/api/vtop/types.dart';

abstract class TimetableRepository {
  Future<TimetableData> getTimetableFromStorage();
  Future<void> saveTimetableToStorage({required TimetableData timetable});
  Future<void> updateTimetable();
}
