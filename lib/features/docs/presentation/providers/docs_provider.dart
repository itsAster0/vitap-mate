import 'package:file_picker/file_picker.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';

Future<String?> pickDocPath() async {
  final file = await FilePicker.pickFile(
    type: FileType.custom,
    allowedExtensions: [
      'pdf',
      'png',
      'jpg',
      'jpeg',
      'webp',
      'gif',
      'bmp',
      'xlsx',
      'xlsm',
      'xls',
      'ods',
      'xml',
      'html',
      'htm',
      'txt',
      'json',
      'md',
      'csv',
    ],
  );
  return file?.path;
}

final docsRepositoryProvider = FutureProvider<DocsRepository>((ref) async {
  final storage = await ref.watch(jsonFileStorageProvider.future);
  return DocsRepository(storage);
});

final docsRegistryProvider =
    AsyncNotifierProvider<DocsRegistryNotifier, List<DocWindow>>(
      DocsRegistryNotifier.new,
    );

final activeDocumentTitleProvider =
    NotifierProvider<ActiveDocumentTitle, String?>(ActiveDocumentTitle.new);

class ActiveDocumentTitle extends Notifier<String?> {
  @override
  String? build() => null;

  void show(String title) => state = title;

  void clear(String title) {
    if (state == title) state = null;
  }
}

class DocsRegistryNotifier extends AsyncNotifier<List<DocWindow>> {
  @override
  Future<List<DocWindow>> build() async {
    final repo = await ref.watch(docsRepositoryProvider.future);
    return repo.list();
  }

  Future<DocsRepository> _repo() => ref.read(docsRepositoryProvider.future);

  Future<void> importNew(String pickedPath, String name) async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      final repo = await _repo();
      final windows = await repo.list();
      windows.add(await repo.importFile(sourcePath: pickedPath, name: name));
      return windows;
    });
  }

  Future<void> fillPreset(DocWindow preset, String pickedPath) async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      final repo = await _repo();
      final updated = await repo.fillPreset(preset.id, pickedPath);
      if (updated == null) return repo.list();
      final windows = await repo.list();
      return windows;
    });
  }

  Future<void> rename(String id, String name) async {
    final repo = await _repo();
    await repo.rename(id, name);
    ref.invalidateSelf();
  }

  Future<void> remove(String id) async {
    final repo = await _repo();
    await repo.remove(id);
    ref.invalidateSelf();
  }

  Future<void> saveScrollState(
    String id, {
    required double scale,
    required double offsetX,
    required double offsetY,
    double scrollOffset = 0.0,
  }) async {
    final repo = await _repo();
    await repo.saveScrollState(
      id,
      scale: scale,
      offsetX: offsetX,
      offsetY: offsetY,
      scrollOffset: scrollOffset,
    );
    final windows = state.value;
    if (windows != null) {
      state = AsyncData([
        for (final window in windows)
          if (window.id == id)
            window.copyWith(
              scale: scale,
              offsetX: offsetX,
              offsetY: offsetY,
              scrollOffset: scrollOffset,
            )
          else
            window,
      ]);
    }
  }

  Future<void> touchLastOpened(String id) async {
    final openedAt = DateTime.now().millisecondsSinceEpoch;
    final windows = state.value;
    if (windows != null) {
      state = AsyncData([
        for (final window in windows)
          if (window.id == id)
            window.copyWith(lastOpenedAt: openedAt)
          else
            window,
      ]);
    }
    final repo = await _repo();
    await repo.touchLastOpened(id, openedAt: openedAt);
  }
}
