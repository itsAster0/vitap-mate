import 'package:vitapmate/features/more/data/datasources/local_data_sources.dart';
import 'package:vitapmate/features/more/data/datasources/remote_data_sources.dart';
import 'package:vitapmate/features/more/domine/repositories/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamScheduleRepoImpl implements ExamScheduleRepository {
  final String semid;
  final ExamScheduleRemoteDataSource remoteDataSource;
  final ExamScheduleLocalDataSource localDataSource;

  ExamScheduleRepoImpl({
    required this.semid,
    required this.remoteDataSource,
    required this.localDataSource,
  });
  @override
  Future<ExamScheduleData> getExamScheduleFromStorage() async {
    var data = await localDataSource.getExamSchedule(semid);
    return data;
  }

  @override
  Future<void> saveExamScheduleToStorage({
    required ExamScheduleData data,
  }) async {
    await localDataSource.saveExamSchedule(data, semid);
  }

  @override
  Future<void> updateExamSchedule() async {
    var data = await remoteDataSource.fetchScheduleeFromRemote(semid);
    if (data.exams.isEmpty) return;
    saveExamScheduleToStorage(data: data);
  }
}
