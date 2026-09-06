import 'package:vitapmate/features/docs/data/doc_models.dart';

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
    final int primary;
    switch (sort) {
      case DocSort.recentlyOpened:
        if (a.lastOpenedAt == null || b.lastOpenedAt == null) {
          primary = a.lastOpenedAt == b.lastOpenedAt
              ? 0
              : a.lastOpenedAt == null
              ? 1
              : -1;
        } else {
          primary = b.lastOpenedAt!.compareTo(a.lastOpenedAt!);
        }
      case DocSort.recentlyAdded:
        primary = b.addedAt.compareTo(a.addedAt);
      case DocSort.nameAscending:
        primary = byName;
      case DocSort.nameDescending:
        primary = -byName;
    }
    if (primary != 0) return primary;
    if (byName != 0) return byName;
    return a.id.compareTo(b.id);
  });
  return result;
}
