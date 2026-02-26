import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/features/more/data/repositories/exam_schedule_repo_impl.dart';
import 'package:vitapmate/features/more/data/repositories/grade_history_repo.dart';
import 'package:vitapmate/features/more/data/repositories/grades_repo.dart';
import 'package:vitapmate/features/more/data/repositories/marks_repo.dart';
import 'package:vitapmate/features/more/domine/repositories/exam_schedule.dart';
import 'package:vitapmate/features/more/domine/repositories/grade_history_repo.dart';
import 'package:vitapmate/features/more/domine/repositories/grades_repo.dart';
import 'package:vitapmate/features/more/domine/repositories/marks_repo.dart';
import 'package:vitapmate/features/more/presentation/providers/state/data_source.dart';
part 'exam_schedule.g.dart';

@riverpod
Future<ExamScheduleRepository> examScheduleRepository(Ref ref) async {
  return ExamScheduleRepoImpl(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    remoteDataSource: await ref.watch(
      examScheduleRemoteDataSourceProvider.future,
    ),
    localDataSource: await ref.watch(
      examScheduleLocalDataSourceProvider.future,
    ),
  );
}

@riverpod
Future<MarksRepository> marksRepository(Ref ref) async {
  return MarksRepoImpl(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    remoteDataSource: await ref.watch(marksRemoteDataSourceProvider.future),
    localDataSource: await ref.watch(marksLocalDataSourceProvider.future),
  );
}

@riverpod
Future<GradesRepository> gradesRepository(Ref ref) async {
  return GradesRepoImpl(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    remoteDataSource: await ref.watch(gradesRemoteDataSourceProvider.future),
    localDataSource: await ref.watch(gradesLocalDataSourceProvider.future),
  );
}

@riverpod
Future<GradesRepository> gradesRepositoryForSem(Ref ref, String semid) async {
  return GradesRepoImpl(
    semid: semid,
    remoteDataSource: await ref.watch(gradesRemoteDataSourceProvider.future),
    localDataSource: await ref.watch(gradesLocalDataSourceProvider.future),
  );
}

@riverpod
Future<GradeHistoryRepository> gradeHistoryRepository(Ref ref) async {
  return GradeHistoryRepoImpl(
    remoteDataSource: await ref.watch(
      gradeHistoryRemoteDataSourceProvider.future,
    ),
    localDataSource: await ref.watch(
      gradeHistoryLocalDataSourceProvider.future,
    ),
  );
}
