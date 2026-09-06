import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/docs/domain/docs_query.dart';

final docsSortProvider = AsyncNotifierProvider<DocsSortNotifier, DocSort>(
  DocsSortNotifier.new,
);

class DocsSortNotifier extends AsyncNotifier<DocSort> {
  static const storageKey = 'docs_preferences';
  Future<void> _pendingWrite = Future.value();

  @override
  Future<DocSort> build() async {
    final storage = await ref.watch(jsonFileStorageProvider.future);
    try {
      final data = await storage.readJson(storageKey);
      return DocSort.fromStored(data?['sort']);
    } catch (_) {
      return DocSort.recentlyOpened;
    }
  }

  Future<void> select(DocSort sort) {
    final storage = ref.read(jsonFileStorageProvider.future);
    state = AsyncData(sort);
    // Serialize rapid selections so an older write cannot overwrite a newer one.
    final write = _pendingWrite.then((_) async {
      await (await storage).writeJson(storageKey, {'sort': sort.name});
    });
    _pendingWrite = write.catchError((Object _) {});
    return write;
  }
}
