import 'package:vitapmate/src/api/vtop/types.dart';

abstract class SemidRepository {
  Future<SemesterData> getSemidsFromStorage();
  Future<SemesterData> updateSemids();
}
