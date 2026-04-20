import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/features/attendance/data/repositories/attendance_repository_impl.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/data_sources_att.dart';

part 'attendance_repository.g.dart';

@Riverpod(keepAlive: true)
Future<AttendanceRepository> attendanceRepository(Ref ref) async {
  return AttendanceRepository(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    dataSource: await ref.read(attendanceDataSourceProvider.future),
  );
}
