import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/domain/docs_query.dart';

void main() {
  const documents = [
    DocWindow(id: 'z', name: 'zebra', kind: DocKind.pdf, addedAt: 30),
    DocWindow(
      id: 'b',
      name: 'Beta',
      kind: DocKind.text,
      addedAt: 20,
      lastOpenedAt: 40,
    ),
    DocWindow(
      id: 'a2',
      name: 'ALPHA',
      kind: DocKind.pdf,
      addedAt: 20,
      lastOpenedAt: 40,
    ),
    DocWindow(
      id: 'a1',
      name: 'alpha',
      kind: DocKind.pdf,
      addedAt: 20,
      lastOpenedAt: 40,
    ),
    DocWindow(
      id: 'm',
      name: 'Mess Menu',
      kind: DocKind.none,
      addedAt: 0,
      isPreset: true,
    ),
  ];

  test(
    'all sort modes have deterministic ties and preserve registry order',
    () {
      const expected = {
        DocSort.recentlyOpened: ['a1', 'a2', 'b', 'm', 'z'],
        DocSort.recentlyAdded: ['z', 'a1', 'a2', 'b', 'm'],
        DocSort.nameAscending: ['a1', 'a2', 'b', 'm', 'z'],
        DocSort.nameDescending: ['z', 'm', 'b', 'a1', 'a2'],
      };
      for (final entry in expected.entries) {
        expect(
          queryDocs(documents, sort: entry.key).map((d) => d.id),
          entry.value,
        );
      }
      expect(documents.map((d) => d.id), ['z', 'b', 'a2', 'a1', 'm']);
    },
  );

  test('recently opened orders timestamps newest first', () {
    final docs = [documents[1], documents[0].copyWith(lastOpenedAt: 100)];
    expect(queryDocs(docs, sort: DocSort.recentlyOpened).first.id, 'z');
  });

  test(
    'search trims, ignores case, matches substrings and includes presets',
    () {
      expect(
        queryDocs(
          documents,
          sort: DocSort.nameAscending,
          query: '  LpH  ',
        ).map((d) => d.id),
        ['a1', 'a2'],
      );
      expect(
        queryDocs(
          documents,
          sort: DocSort.recentlyOpened,
          query: 'menu',
        ).single.id,
        'm',
      );
      expect(
        queryDocs(documents, sort: DocSort.recentlyAdded, query: 'missing'),
        isEmpty,
      );
      expect(
        queryDocs(documents, sort: DocSort.recentlyAdded, query: '  '),
        hasLength(5),
      );
    },
  );
}
