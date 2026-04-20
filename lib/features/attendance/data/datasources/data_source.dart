import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

class AttendanceDataSource {
  final JsonFileStorage _storage;
  final Future<VtopClient> Function() _client;
  final AsyncQueue _globalAsyncQueue;

  AttendanceDataSource(this._storage, this._client, this._globalAsyncQueue);

  Future<FullAttendanceData> getFullAttendance(
    String semid,
    String courseId,
    String courseType,
  ) async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_fullattendance_${semid}_${courseType}_$courseId',
      () async {
        final payload = await _storage.readJson(
          'full_attendance_${semid}_${courseType}_$courseId',
        );
        if (payload == null) return null;
        return FullAttendanceData.fromJson(payload);
      },
    );

    return data ??
        FullAttendanceData(
          records: const [],
          semesterId: '',
          updateTime: BigInt.zero,
          courseId: courseId,
          courseType: courseType,
        );
  }

  Future<void> saveFullAttendance(
    FullAttendanceData attendance,
    String semid,
    String courseType,
    String courseId,
  ) async {
    await _globalAsyncQueue.run(
      'toStorage_fullattendance_$semid',
      () => _storage.writeJson(
        'full_attendance_${semid}_${courseType}_$courseId',
        attendance.toJson(),
      ),
    );
  }

  Future<AttendanceData> getAttendance(String semid) async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_attendance_$semid',
      () async {
        final payload = await _storage.readJson('attendance_$semid');
        if (payload == null) return null;
        return AttendanceData.fromJson(payload);
      },
    );

    return data ??
        AttendanceData(
          records: const [],
          semesterId: '',
          updateTime: BigInt.zero,
        );
  }

  Future<void> saveAttendance(AttendanceData attendance, String semid) async {
    await _globalAsyncQueue.run(
      'toStorage_attendance_$semid',
      () => _storage.writeJson('attendance_$semid', attendance.toJson()),
    );
  }

  Future<AttendanceData> fetchAttendance(String semid) async {
    return AppLogger.instance.trackRequest(
      source: 'client.attendance',
      action: 'fetchAttendance semid=$semid',
      run: () => _globalAsyncQueue.run(
        'vtop_attendance_$semid',
        () async => vtop_api.fetchAttendance(
          client: await _client(),
          semesterId: semid,
        ),
      ),
    );
  }

  Future<FullAttendanceData> fetchFullAttendance(
    String semid,
    String courseType,
    String courseId,
  ) async {
    return AppLogger.instance.trackRequest(
      source: 'client.attendance',
      action: 'fetchFullAttendance semid=$semid courseId=$courseId',
      run: () => _globalAsyncQueue.run(
        'vtop_fullattendance_${semid}_${courseType}_$courseId',
        () async => vtop_api.fetchFullAttendance(
          client: await _client(),
          semesterId: semid,
          courseId: courseId,
          courseType: courseType,
        ),
      ),
    );
  }
}
