import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/features/more/data/repositories/exam_schedule_repo_impl.dart';
import 'package:vitapmate/features/more/data/repositories/grade_history_repo.dart';
import 'package:vitapmate/features/more/data/repositories/grades_repo.dart';
import 'package:vitapmate/features/more/data/repositories/marks_repo.dart';
import 'package:vitapmate/features/more/presentation/providers/state/data_source.dart';

part 'exam_schedule.g.dart';

@Riverpod(keepAlive: true)
Future<ExamScheduleRepository> examScheduleRepository(Ref ref) async {
  return ExamScheduleRepository(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    dataSource: await ref.watch(examScheduleDataSourceProvider.future),
  );
}

@Riverpod(keepAlive: true)
Future<MarksRepository> marksRepository(Ref ref) async {
  return MarksRepository(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    dataSource: await ref.watch(marksDataSourceProvider.future),
  );
}

@Riverpod(keepAlive: true)
Future<GradesRepository> gradesRepository(Ref ref) async {
  return GradesRepository(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    dataSource: await ref.watch(gradesDataSourceProvider.future),
  );
}

@Riverpod(keepAlive: true)
Future<GradesRepository> gradesRepositoryForSem(Ref ref, String semid) async {
  return GradesRepository(
    semid: semid,
    dataSource: await ref.watch(gradesDataSourceProvider.future),
  );
}

@Riverpod(keepAlive: true)
Future<GradeHistoryRepository> gradeHistoryRepository(Ref ref) async {
  return GradeHistoryRepository(
    dataSource: await ref.watch(gradeHistoryDataSourceProvider.future),
  );
}
