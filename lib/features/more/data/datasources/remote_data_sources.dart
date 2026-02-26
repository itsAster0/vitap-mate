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
      "vtop_fetchSchedule_$semid",
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

class GradesRemoteDataSource {
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;
  GradesRemoteDataSource(this._client, this._globalAsyncQueue);

  Future<GradeViewData> fetchGradeViewFromRemote(String semid) async {
    final data = await _globalAsyncQueue.run(
      "vtop_grades_$semid",
      () => fetchGradeView(client: _client, semesterId: semid),
    );
    return data;
  }

  Future<GradeDetailsData> fetchGradeDetailsFromRemote({
    required String semid,
    required String courseId,
  }) async {
    final data = await _globalAsyncQueue.run(
      "vtop_grade_details_${semid}_$courseId",
      () => fetchGradeViewDetails(
        client: _client,
        semesterId: semid,
        courseId: courseId,
      ),
    );
    return data;
  }
}

class GradeHistoryRemoteDataSource {
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;
  GradeHistoryRemoteDataSource(this._client, this._globalAsyncQueue);

  Future<GradeHistoryData> fetchGradeHistoryFromRemote() async {
    final data = await _globalAsyncQueue.run(
      "vtop_grade_history",
      () => fetchGradeHistory(client: _client),
    );
    return data;
  }
}
