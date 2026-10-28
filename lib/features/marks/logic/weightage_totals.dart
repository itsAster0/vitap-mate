class WeightageTotals {
  final double gained;
  final double possible;

  const WeightageTotals({required this.gained, required this.possible});

  double get percentage => possible > 0 ? (gained / possible) * 100 : 0.0;
}

/// Weighted marks scored and assessed so far. A re-evaluation counts as the
/// assessment it re-marks, and only the better of the two is kept.
WeightageTotals calculateWeightageTotals<T>(
  Iterable<T> items, {
  required String Function(T item) titleOf,
  required String Function(T item) gainedOf,
  required String Function(T item) possibleOf,
}) {
  final reEvalPattern = RegExp(r'\bre[\s-]*evaluation\b', caseSensitive: false);
  String normalize(String input) =>
      input.toLowerCase().replaceAll(RegExp(r'\s+'), ' ').trim();
  double parseNum(String raw) =>
      double.tryParse(raw.replaceAll('%', '').trim()) ?? 0.0;
  void keepMax(Map<String, double> byKey, String key, double value) {
    final old = byKey[key];
    if (old == null || value > old) byKey[key] = value;
  }

  final gainedByKey = <String, double>{};
  final possibleByKey = <String, double>{};
  for (final item in items) {
    final title = normalize(titleOf(item));
    if (title.isEmpty) continue;
    final baseTitle = normalize(title.replaceAll(reEvalPattern, ''));
    final key = reEvalPattern.hasMatch(title) && baseTitle.isNotEmpty
        ? baseTitle
        : title;
    keepMax(gainedByKey, key, parseNum(gainedOf(item)));
    keepMax(possibleByKey, key, parseNum(possibleOf(item)));
  }

  double sum(Map<String, double> byKey) =>
      byKey.values.fold(0.0, (total, v) => total + v);
  return WeightageTotals(
    gained: sum(gainedByKey),
    possible: sum(possibleByKey),
  );
}
