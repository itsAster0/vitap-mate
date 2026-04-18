import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

class ExamScheduleDataSource {
  final JsonFileStorage _storage;
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;

  ExamScheduleDataSource(this._storage, this._client, this._globalAsyncQueue);

  Future<ExamScheduleData> getExamSchedule(String semid) async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_exam_schedule_$semid',
      () async {
        final payload = await _storage.readJson('exam_schedule_$semid');
        if (payload == null) return null;
        return ExamScheduleData.fromJson(payload);
      },
    );
    return data ??
        ExamScheduleData(
          exams: const [],
          semesterId: '',
          updateTime: BigInt.zero,
        );
  }

  Future<void> saveExamSchedule(ExamScheduleData data, String semid) async {
    await _globalAsyncQueue.run(
      'toStorage_exam_schedule_$semid',
      () => _storage.writeJson('exam_schedule_$semid', data.toJson()),
    );
  }

  Future<ExamScheduleData> fetchExamSchedule(String semid) async {
    return AppLogger.instance.trackRequest(
      source: 'client.exam_schedule',
      action: 'fetchExamSchedule semid=$semid',
      run: () => _globalAsyncQueue.run(
        'vtop_fetchSchedule_$semid',
        () => vtop_api.fetchExamShedule(client: _client, semesterId: semid),
      ),
    );
  }
}

class MarksDataSource {
  final JsonFileStorage _storage;
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;

  MarksDataSource(this._storage, this._client, this._globalAsyncQueue);

  Future<MarksData> getMarks(String semid) async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_marks_$semid',
      () async {
        final payload = await _storage.readJson('marks_$semid');
        if (payload == null) return null;
        return MarksData.fromJson(payload);
      },
    );
    return data ??
        MarksData(records: const [], semesterId: '', updateTime: BigInt.zero);
  }

  Future<void> saveMarks(MarksData data, String semid) async {
    await _globalAsyncQueue.run(
      'toStorage_marks_$semid',
      () => _storage.writeJson('marks_$semid', data.toJson()),
    );
  }

  Future<MarksData> fetchMarks(String semid) async {
    return AppLogger.instance.trackRequest(
      source: 'client.marks',
      action: 'fetchMarks semid=$semid',
      run: () => _globalAsyncQueue.run(
        'vtop_marks_$semid',
        () => vtop_api.fetchMarks(client: _client, semesterId: semid),
      ),
    );
  }
}

class GradesDataSource {
  final JsonFileStorage _storage;
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;

  GradesDataSource(this._storage, this._client, this._globalAsyncQueue);

  Future<GradeViewData> getGradeView(String semid) async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_grades_view_$semid',
      () async {
        final payload = await _storage.readJson('grades_view_$semid');
        if (payload == null) return null;
        return GradeViewData.fromJson(payload);
      },
    );
    return data ??
        GradeViewData(
          courses: const [],
          semesters: const [],
          semesterId: '',
          updateTime: BigInt.zero,
        );
  }

  Future<Map<String, GradeDetailsData>> getGradeDetailsMap(String semid) async {
    final payload = await _globalAsyncQueue.run(
      'fromStorage_grades_details_$semid',
      () => _storage.readJson('grades_details_$semid'),
    );
    if (payload == null) return {};
    return {
      for (final entry in payload.entries)
        entry.key: GradeDetailsData.fromJson(
          Map<String, dynamic>.from(entry.value as Map),
        ),
    };
  }

  Future<void> saveGradeView(GradeViewData data, String semid) async {
    await _globalAsyncQueue.run(
      'toStorage_grades_view_$semid',
      () => _storage.writeJson('grades_view_$semid', data.toJson()),
    );
  }

  Future<void> saveGradeDetails(GradeDetailsData data, String semid) async {
    await _globalAsyncQueue.run(
      'toStorage_grades_details_${semid}_${data.courseId}',
      () async {
        final current = await _storage.readJson('grades_details_$semid') ?? {};
        current[data.courseId] = data.toJson();
        await _storage.writeJson('grades_details_$semid', current);
      },
    );
  }

  Future<GradeViewData> fetchGradeView(String semid) async {
    return AppLogger.instance.trackRequest(
      source: 'client.grades',
      action: 'fetchGradeView semid=$semid',
      run: () => _globalAsyncQueue.run(
        'vtop_grades_$semid',
        () => vtop_api.fetchGradeView(client: _client, semesterId: semid),
      ),
    );
  }

  Future<GradeDetailsData> fetchGradeDetails({
    required String semid,
    required String courseId,
  }) async {
    return AppLogger.instance.trackRequest(
      source: 'client.grades',
      action: 'fetchGradeDetails semid=$semid courseId=$courseId',
      run: () => _globalAsyncQueue.run(
        'vtop_grade_details_${semid}_$courseId',
        () => vtop_api.fetchGradeViewDetails(
          client: _client,
          semesterId: semid,
          courseId: courseId,
        ),
      ),
    );
  }
}

class GradeHistoryDataSource {
  final JsonFileStorage _storage;
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;

  GradeHistoryDataSource(this._storage, this._client, this._globalAsyncQueue);

  Future<GradeHistoryData> getGradeHistory() async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_grade_history',
      () async {
        final payload = await _storage.readJson('grade_history');
        if (payload == null) return null;
        return GradeHistoryData.fromJson(payload);
      },
    );
    return data ??
        GradeHistoryData(
          student: GradeHistoryStudentInfo(
            regNo: '',
            name: '',
            programmeBranch: '',
            programmeMode: '',
            studySystem: '',
            gender: '',
            yearJoined: '',
            eduStatus: '',
            school: '',
            campus: '',
          ),
          records: const [],
          cgpa: GradeHistoryCgpa(
            creditsRegistered: '',
            creditsEarned: '',
            cgpa: '',
            sGrades: '',
            aGrades: '',
            bGrades: '',
            cGrades: '',
            dGrades: '',
            eGrades: '',
            fGrades: '',
            nGrades: '',
          ),
          updateTime: BigInt.zero,
        );
  }

  Future<void> saveGradeHistory(GradeHistoryData data) async {
    await _globalAsyncQueue.run(
      'toStorage_grade_history',
      () => _storage.writeJson('grade_history', data.toJson()),
    );
  }

  Future<GradeHistoryData> fetchGradeHistory() async {
    return AppLogger.instance.trackRequest(
      source: 'client.grade_history',
      action: 'fetchGradeHistory',
      run: () => _globalAsyncQueue.run(
        'vtop_grade_history',
        () => vtop_api.fetchGradeHistory(client: _client),
      ),
    );
  }
}
