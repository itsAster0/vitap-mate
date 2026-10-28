/// One VTOP page: the saved copy first, then VTOP through [VtopBackend].
/// Replaces a data source, repository, state providers and controller per
/// page. A feature declares its page in one line:
///
/// ```dart
/// final marksProvider = vtopPage(VtopQueries.marks);
/// ```
///
/// or subclasses [VtopPage] when the page needs more (see attendance). A
/// screen with its own state calls [loadPage] and [fetchPage] directly.
library;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show ProviderListenable;
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/platform/storage/json_file_storage_provider.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/vtop/clinet_provider.dart';
import 'package:vitapmate/vtop/global_async_queue_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';

/// Reads a provider: `ref.read` of a [Ref] or a [WidgetRef], or a
/// container's `read`.
typedef Read = T Function<T>(ProviderListenable<T> provider);

/// A page for the selected semester.
AsyncNotifierProvider<VtopPage<T>, T> vtopPage<T>(
  VtopQuery<T> Function(String semester) query, {
  void Function(Ref ref, T data)? onData,
}) => AsyncNotifierProvider(() => VtopPage(query, onData: onData));

/// A page that is not per semester (profile, grade history...).
AsyncNotifierProvider<VtopPage<T>, T> vtopGlobalPage<T>(
  VtopQuery<T> Function() query,
) => AsyncNotifierProvider(() => VtopPage((_) => query(), perSemester: false));

/// The selected semester's id.
final selectedSemesterProvider = FutureProvider(
  (ref) => ref.watch(vtopUserProvider.selectAsync((user) => user.semid!)),
);

class VtopPage<T> extends AsyncNotifier<T> {
  VtopPage(
    this._query, {
    bool perSemester = true,
    void Function(Ref ref, T data)? onData,
  }) : _perSemester = perSemester,
       _onData = onData;

  final VtopQuery<T> Function(String semester) _query;
  final bool _perSemester;

  /// Runs on every value, cached or fetched.
  final void Function(Ref ref, T data)? _onData;

  @override
  Future<T> build() async {
    final semester = _perSemester
        ? await ref.watch(selectedSemesterProvider.future)
        : '';
    final data = await loadPage(ref, _query(semester));
    _onData?.call(ref, data);
    return data;
  }

  /// Fetches from VTOP, saves and publishes the page.
  Future<void> refresh() async {
    final data = await fetchPage(ref, await _current());
    state = AsyncData(data);
    _onData?.call(ref, data);
  }

  /// The saved copy, without asking VTOP; null if never fetched.
  Future<T?> cached() async => readSavedPage(ref.read, await _current());

  Future<VtopQuery<T>> _current() async => _query(
    _perSemester ? await ref.read(selectedSemesterProvider.future) : '',
  );
}

/// The saved copy of [query], or VTOP's when there is none.
Future<T> loadPage<T>(Ref ref, VtopQuery<T> query) async =>
    await readSavedPage(ref.read, query) ?? await fetchPage(ref, query);

/// Fetches [query] from VTOP (one request at a time per page) and saves it.
Future<T> fetchPage<T>(Ref ref, VtopQuery<T> query) async {
  final key = savedPageKey(query);
  final data = await flow(
    'fetch $key',
    source: 'vtop.page',
    () =>
        ref.read(globalAsyncQueueProvider.notifier).run('vtop_$key', () async {
          await ref.read(vClientProvider.notifier).ensureLogin();
          return ref.read(vtopBackendProvider).fetch(query);
        }),
  );
  await writeSavedPage(ref.read, query, data);
  return data;
}

/// Where a page is saved: name, semester and argument values.
String savedPageKey(VtopQuery<Object?> query) =>
    [query.name, ?query.semester, ...query.args.values].join('_');

/// The saved copy of [query], or null. Never asks VTOP. A copy saved in
/// an older format reads as missing, so it is fetched again.
Future<T?> readSavedPage<T>(Read read, VtopQuery<T> query) async {
  final storage = await read(jsonFileStorageProvider.future);
  final json = await storage.readJson(savedPageKey(query));
  if (json == null) return null;
  try {
    return query.fromJson(json);
  } catch (_) {
    return null;
  }
}

/// The saved copy of a page for the selected semester. Never asks VTOP.
Future<T?> readSavedSemesterPage<T>(
  Read read,
  VtopQuery<T> Function(String semester) query,
) async =>
    readSavedPage(read, query(await read(selectedSemesterProvider.future)));

/// Saves [data] as the copy of [query].
Future<void> writeSavedPage<T>(Read read, VtopQuery<T> query, T data) async {
  final storage = await read(jsonFileStorageProvider.future);
  // Every page type is generated with toJson.
  await storage.writeJson(savedPageKey(query), (data as dynamic).toJson());
}
