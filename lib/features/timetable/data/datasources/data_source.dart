import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';

class TimetableDataSource {
  final JsonFileStorage _storage;
  final VtopBackend Function() _backend;
  final AsyncQueue _globalAsyncQueue;

  TimetableDataSource(this._storage, this._backend, this._globalAsyncQueue);

  Future<TimetableData> getTimetable(String semid) async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_timetable_$semid',
      () async {
        final payload = await _storage.readJson('timetable_$semid');
        if (payload == null) return null;
        payload.putIfAbsent('courses', () => <dynamic>[]);
        // Timetables cached before course-credit support do not contain this
        // field. Keep those caches readable until the next refresh.
        for (final slot in (payload['slots'] as List<dynamic>? ?? const [])) {
          if (slot is Map<String, dynamic>) {
            slot.putIfAbsent('credits', () => '');
          }
        }
        return TimetableData.fromJson(payload);
      },
    );

    return data ??
        TimetableData(
          slots: const [],
          courses: const [],
          semesterId: '',
          updateTime: BigInt.zero,
        );
  }

  Future<void> saveTimetable(TimetableData timetable, String semid) async {
    await _globalAsyncQueue.run(
      'toStorage_timetable_$semid',
      () => _storage.writeJson('timetable_$semid', timetable.toJson()),
    );
  }

  Future<TimetableData> fetchTimetable(String semid) async {
    return AppLogger.instance.trackRequest(
      source: 'client.timetable',
      action: 'fetchTimetable semid=$semid',
      run: () => _globalAsyncQueue.run(
        'vtop_timetable_$semid',
        () async => _backend().timetable(semid),
      ),
    );
  }
}
