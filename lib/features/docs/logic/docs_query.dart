import 'package:vitapmate/features/docs/logic/doc_models.dart';

enum DocSort {
  recentlyOpened('Recently opened'),
  recentlyAdded('Recently added'),
  nameAscending('Name A–Z'),
  nameDescending('Name Z–A');

  final String label;
  const DocSort(this.label);

  static DocSort fromStored(Object? value) => values.firstWhere(
    (sort) => sort.name == value,
    orElse: () => recentlyOpened,
  );
}

List<DocWindow> queryDocs(
  List<DocWindow> documents, {
  required DocSort sort,
  String query = '',
}) {
  final normalized = query.trim().toLowerCase();
  final result = documents
      .where((doc) => doc.name.toLowerCase().contains(normalized))
      .toList();
  result.sort((a, b) {
    final byName = a.name.toLowerCase().compareTo(b.name.toLowerCase());
    final primary = switch (sort) {
      // Never-opened documents go last.
      .recentlyOpened => switch ((a.lastOpenedAt, b.lastOpenedAt)) {
        (final x?, final y?) => y.compareTo(x),
        (null, null) => 0,
        (null, _) => 1,
        _ => -1,
      },
      .recentlyAdded => b.addedAt.compareTo(a.addedAt),
      .nameAscending => byName,
      .nameDescending => -byName,
    };
    if (primary != 0) return primary;
    if (byName != 0) return byName;
    return a.id.compareTo(b.id);
  });
  return result;
}
