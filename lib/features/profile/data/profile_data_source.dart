import 'package:vitapmate/vtop/global_async_queue_provider.dart';
import 'package:vitapmate/observability/app_logger.dart';
import 'package:vitapmate/platform/storage/json_file_storage.dart';
import 'package:vitapmate/vtop/cached_repository.dart';
import 'package:vitapmate/vtop/backend/vtop_backend.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// A page that is not per semester, saved under [key] and fetched with
/// [query].
class SavedPage<T> {
  SavedPage(
    this._storage,
    this._backend,
    this._globalAsyncQueue, {
    required this.key,
    required this.query,
    required this.fromJson,
    required this.toJson,
  });

  final JsonFileStorage _storage;
  final VtopBackend Function() _backend;
  final GlobalAsyncQueue _globalAsyncQueue;
  final String key;
  final VtopQuery<T> Function() query;
  final T Function(Map<String, dynamic>) fromJson;
  final Map<String, dynamic> Function(T) toJson;

  Future<T?> read() => _globalAsyncQueue.run('fromStorage_$key', () async {
    final payload = await _storage.readJson(key);
    return payload == null ? null : fromJson(payload);
  });

  Future<void> write(T data) => _globalAsyncQueue.run(
    'toStorage_$key',
    () => _storage.writeJson(key, toJson(data)),
  );

  Future<T> fetch() => AppLogger.instance.trackRequest(
    source: 'client.$key',
    action: 'fetch $key',
    run: () => _globalAsyncQueue.run(
      'vtop_$key',
      () async => _backend().fetch(query()),
    ),
  );
}

class ProfileDataSource {
  ProfileDataSource(
    JsonFileStorage storage,
    VtopBackend Function() backend,
    GlobalAsyncQueue globalAsyncQueue,
  ) : profile = SavedPage(
        storage,
        backend,
        globalAsyncQueue,
        key: 'student_profile',
        query: VtopQueries.profile,
        fromJson: StudentProfile.fromJson,
        toJson: (data) => data.toJson(),
      ),
      mentor = SavedPage(
        storage,
        backend,
        globalAsyncQueue,
        key: 'mentor',
        query: VtopQueries.mentor,
        fromJson: MentorData.fromJson,
        toJson: (data) => data.toJson(),
      );

  final SavedPage<StudentProfile> profile;
  final SavedPage<MentorData> mentor;
}

class SavedPageRepository<T> extends CachedRepository<T> {
  SavedPageRepository(this._page);

  final SavedPage<T> _page;

  @override
  Future<T?> loadCache() => _page.read();

  @override
  Future<void> saveCache(T data) => _page.write(data);

  @override
  Future<T> fetchRemote() => _page.fetch();
}
