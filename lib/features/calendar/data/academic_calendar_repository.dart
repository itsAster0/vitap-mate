import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AcademicCalendarDataSource {
  final JsonFileStorage _storage;
  final VtopBackend Function() _backend;
  final GlobalAsyncQueue _globalAsyncQueue;

  AcademicCalendarDataSource(
    this._storage,
    this._backend,
    this._globalAsyncQueue,
  );

  Future<AcademicCalendarData?> getCalendar(String semid) {
    return _globalAsyncQueue.run(
      'fromStorage_academic_calendar_$semid',
      () async {
        final payload = await _storage.readJson('academic_calendar_$semid');
        if (payload == null) return null;
        return AcademicCalendarData.fromJson(payload);
      },
    );
  }

  Future<void> saveCalendar(AcademicCalendarData data, String semid) async {
    await _globalAsyncQueue.run(
      'toStorage_academic_calendar_$semid',
      () => _storage.writeJson('academic_calendar_$semid', data.toJson()),
    );
  }

  Future<AcademicCalendarData> fetchCalendar(String semid) {
    return AppLogger.instance.trackRequest(
      source: 'client.academic_calendar',
      action: 'fetchAcademicCalendar semid=$semid',
      run: () => _globalAsyncQueue.run(
        'vtop_fetchAcademicCalendar_$semid',
        () async => _backend().academicCalendar(semid),
      ),
    );
  }
}

class AcademicCalendarRepository
    extends CachedRepository<AcademicCalendarData> {
  final String semid;
  final AcademicCalendarDataSource _dataSource;

  AcademicCalendarRepository({
    required this.semid,
    required AcademicCalendarDataSource dataSource,
  }) : _dataSource = dataSource;

  @override
  Future<AcademicCalendarData?> loadCache() => _dataSource.getCalendar(semid);

  @override
  Future<void> saveCache(AcademicCalendarData data) =>
      _dataSource.saveCalendar(data, semid);

  @override
  Future<AcademicCalendarData> fetchRemote() =>
      _dataSource.fetchCalendar(semid);
}
