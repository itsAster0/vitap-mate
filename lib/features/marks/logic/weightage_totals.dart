class WeightageTotals {
  final double gained;
  final double possible;

  const WeightageTotals({required this.gained, required this.possible});

  double get percentage => possible > 0 ? (gained / possible) * 100 : 0.0;
}

WeightageTotals calculateWeightageTotals<T>(
  Iterable<T> items, {
  required String Function(T item) titleOf,
  required String Function(T item) gainedOf,
  required String Function(T item) possibleOf,
}) {
  final reEvalPattern = RegExp(r'\bre[\s-]*evaluation\b', caseSensitive: false);

  String normalize(String input) =>
      input.toLowerCase().replaceAll(RegExp(r'\s+'), ' ').trim();

  double parseNum(String raw) {
    final cleaned = raw.replaceAll('%', '').trim();
    return double.tryParse(cleaned) ?? 0.0;
  }

  final gainedByKey = <String, double>{};
  final possibleByKey = <String, double>{};
  double totalGained = 0.0;
  double totalPossible = 0.0;

  for (final item in items) {
    final titleNorm = normalize(titleOf(item));
    if (titleNorm.isEmpty) continue;

    final isReEval = reEvalPattern.hasMatch(titleNorm);
    final baseTitle = normalize(titleNorm.replaceAll(reEvalPattern, ''));
    final key = isReEval && baseTitle.isNotEmpty ? baseTitle : titleNorm;

    final gained = parseNum(gainedOf(item));
    final possible = parseNum(possibleOf(item));

    final oldGained = gainedByKey[key];
    final oldPossible = possibleByKey[key];

    if (oldGained == null || gained > oldGained) {
      if (oldGained != null) totalGained -= oldGained;
      gainedByKey[key] = gained;
      totalGained += gained;
    }

    if (oldPossible == null || possible > oldPossible) {
      if (oldPossible != null) totalPossible -= oldPossible;
      possibleByKey[key] = possible;
      totalPossible += possible;
    }
  }

  return WeightageTotals(gained: totalGained, possible: totalPossible);
}
