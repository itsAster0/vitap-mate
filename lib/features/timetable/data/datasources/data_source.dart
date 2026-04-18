import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

class TimetableDataSource {
  final JsonFileStorage _storage;
  final VtopClient _client;
  final AsyncQueue _globalAsyncQueue;

  TimetableDataSource(this._storage, this._client, this._globalAsyncQueue);

  Future<TimetableData> getTimetable(String semid) async {
    final data = await _globalAsyncQueue.run(
      'fromStorage_timetable_$semid',
      () async {
        final payload = await _storage.readJson('timetable_$semid');
        if (payload == null) return null;
        return TimetableData.fromJson(payload);
      },
    );

    return data ??
        TimetableData(slots: const [], semesterId: '', updateTime: BigInt.zero);
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
        () => vtop_api.fetchTimetable(client: _client, semesterId: semid),
      ),
    );
  }
}
