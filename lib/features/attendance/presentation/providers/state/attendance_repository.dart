import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/features/attendance/data/repositories/attendance_repository_impl.dart';
import 'package:vitapmate/features/attendance/domine/repositories/attendance_repository.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/data_sources_att.dart';

part 'attendance_repository.g.dart';

@riverpod
Future<AttendanceRepository> attendanceRepository(Ref ref) async {
  var remote = await ref.read(attendanceRemoteDataSourceProvider.future);
  var local = await ref.read(attendanceLocalDataSourceProvider.future);
  return AttendanceRepositoryImpl(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    remoteDataSource: remote,
    localDataSource: local,
  );
}
