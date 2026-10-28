import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/platform/storage/json_file_storage_provider.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
import 'package:vitapmate/vtop/global_async_queue_provider.dart';
import 'package:vitapmate/platform/storage/json_file_storage.dart';
import 'package:vitapmate/features/more/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

import '../support/slow_storage.dart';

FullAttendanceData _history(String courseId) => FullAttendanceData(
  records: const [],
  semesterId: 'S',
  updateTime: BigInt.one,
  courseId: courseId,
  courseType: 'ETH',
);

GradeDetailsData _details(String courseId) => GradeDetailsData(
  semesterId: 'S',
  courseId: courseId,
  classNumber: '',
  classCourseType: '',
  grandTotal: '',
  marks: const [],
  gradeRanges: const [],
  updateTime: BigInt.one,
);

void main() {
  late ProviderContainer container;
  late GlobalAsyncQueue queue;
  late SlowStorage storage;
  setUp(() {
    container = ProviderContainer();
    queue = container.read(globalAsyncQueueProvider.notifier);
    storage = SlowStorage();
  });
  tearDown(() => container.dispose());

  test('saving two courses\' history at once keeps both', () async {
    final pages = ProviderContainer(
      overrides: [jsonFileStorageProvider.overrideWith((ref) async => storage)],
    );
    addTearDown(pages.dispose);
    VtopQuery<FullAttendanceData> query(String course) =>
        VtopQueries.fullAttendance('S', courseId: course, courseType: 'ETH');
    await Future.wait([
      writeSavedPage(pages.read, query('A'), _history('A')),
      writeSavedPage(pages.read, query('B'), _history('B')),
    ]);
    expect((await readSavedPage(pages.read, query('A')))?.courseId, 'A');
    expect((await readSavedPage(pages.read, query('B')))?.courseId, 'B');
  });

  test('saving two courses\' grade details at once keeps both', () async {
    final source = GradesDataSource(
      storage,
      () => throw UnimplementedError(),
      queue,
    );
    await Future.wait([
      source.saveGradeDetails(_details('A'), 'S'),
      source.saveGradeDetails(_details('B'), 'S'),
    ]);
    expect((await source.getGradeDetailsMap('S')).keys.toSet(), {'A', 'B'});
  });

  group('JsonFileStorage on disk', () {
    late Directory root;
    late JsonFileStorage files;
    setUp(() async {
      root = await Directory.systemTemp.createTemp('vitapmate-storage');
      files = JsonFileStorage(username: 'u', root: () async => root);
    });
    tearDown(() => root.delete(recursive: true));

    test('a truncated file reads as missing, not as an error', () async {
      await files.writeJson('attendance_S', {'semesterId': 'S'});
      final saved = (await root.list(recursive: true).toList())
          .whereType<File>()
          .single;
      await saved.writeAsString('{"semesterId": "S", "rec');
      expect(await files.readJson('attendance_S'), isNull);
    });

    test('writing leaves only the saved file behind', () async {
      await files.writeJson('marks_S', {'a': 1});
      await files.writeJson('marks_S', {'a': 2});
      final names = (await root.list(recursive: true).toList())
          .whereType<File>()
          .map((f) => f.uri.pathSegments.last);
      expect(names, ['marks_S.json']);
      expect(await files.readJson('marks_S'), {'a': 2});
    });
  });
}
