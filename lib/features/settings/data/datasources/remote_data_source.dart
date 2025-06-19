import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class SemesterIdRemoteDataSource {
  final GlobalAsyncQueue _globalAsyncQueue;
  final VtopClient _client;
  SemesterIdRemoteDataSource(this._globalAsyncQueue, this._client);
  Future<SemesterData> fetchSemidsFromremote() async {
    var data = await _globalAsyncQueue.run(
      "vtop_semidsfrom_timetabel",
      () => fetchSemesters(client: _client),
    );
    return data;
  }
}
