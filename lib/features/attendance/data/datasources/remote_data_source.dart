import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class AttendanceRemoteDataSource {
  final VtopClient _client;
  final AsyncQueue _globalAsyncQueue;
  AttendanceRemoteDataSource(this._client, this._globalAsyncQueue);

  Future<FullAttendanceData> fetchFullAttendanceFromRemote(
    String semid,
    String courseType,
    String courseId,
  ) async {
    var data = await _globalAsyncQueue.run(
      "vtop_fullattendance_${semid}_${courseType}_$courseId",
      () => fetchFullAttendance(
        client: _client,
        semesterId: semid,
        courseId: courseId,
        courseType: courseType,
      ),
    );
    return data;
  }

  Future<AttendanceData> fetchAttendanceFromRemote(String semid) async {
    var data = await _globalAsyncQueue.run(
      "vtop_attendance_$semid",
      () => fetchAttendance(client: _client, semesterId: semid),
    );
    return data;
  }
}
