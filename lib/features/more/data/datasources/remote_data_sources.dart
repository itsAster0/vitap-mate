import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class ExamScheduleRemoteDataSource {
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;
  ExamScheduleRemoteDataSource(this._client, this._globalAsyncQueue);

  Future<ExamScheduleData> fetchScheduleeFromRemote(String semid) async {
    var data = await _globalAsyncQueue.run(
      "vtop_fetchSchedulee_$semid",
      () => fetchExamShedule(client: _client, semesterId: semid),
    );
    return data;
  }
}

class MarksRemoteDataSource {
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;
  MarksRemoteDataSource(this._client, this._globalAsyncQueue);
  Future<MarksData> fetchMarksFromRemote(String semid) async {
    var data = await _globalAsyncQueue.run(
      "vtop_marks_$semid",
      () => fetchMarks(client: _client, semesterId: semid),
    );
    return data;
  }
}
