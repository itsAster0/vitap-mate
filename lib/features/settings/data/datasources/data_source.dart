import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

class SemesterIdDataSource {
  final JsonFileStorage _storage;
  final VtopClient _client;
  final GlobalAsyncQueue _globalAsyncQueue;

  SemesterIdDataSource(this._storage, this._client, this._globalAsyncQueue);

  Future<SemesterData> getSemidsFromStorage() async {
    final data = await _globalAsyncQueue.run('get_semids_storage', () async {
      final payload = await _storage.readJson('semester_ids');
      if (payload == null) return null;
      return SemesterData.fromJson(payload);
    });

    return data ?? SemesterData(semesters: const [], updateTime: BigInt.zero);
  }

  Future<void> saveSemidsToStorage(SemesterData semid) async {
    await _globalAsyncQueue.run(
      'toStorage_semids',
      () => _storage.writeJson('semester_ids', semid.toJson()),
    );
  }

  Future<SemesterData> fetchSemids() async {
    return AppLogger.instance.trackRequest(
      source: 'client.semesters',
      action: 'fetchSemesters',
      run: () => _globalAsyncQueue.run(
        'vtop_semidsfrom_timetabel',
        () => vtop_api.fetchSemesters(client: _client),
      ),
    );
  }
}
