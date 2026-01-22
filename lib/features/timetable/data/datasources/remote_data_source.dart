import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class TimetableRemoteDataSource {
  final VtopClient _client;
  final AsyncQueue _globalAsyncQueue;
  TimetableRemoteDataSource(this._client, this._globalAsyncQueue);

  Future<TimetableData> fetchTimetableFromRemote(String semid) async {
    final TimetableData data = await _globalAsyncQueue.run(
      "vtop_timetable_$semid",
      () => fetchTimetable(client: _client, semesterId: semid),
    );
    return data;
  }
}
