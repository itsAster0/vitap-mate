/// Reads the VIT-AP monthly mess menu spreadsheet: one sheet per menu
/// (e.g. "Veg & Non-Veg", "Special"), a header row of meals, and day blocks
/// whose first cell names the weekday and the dates it covers
/// ("Thu\n1, 15, 29"). The file itself is never changed.
library;

import 'package:intl/intl.dart';

enum Meal { breakfast, lunch, snacks, dinner }

extension MealLabel on Meal {
  String get label => switch (this) {
    Meal.breakfast => 'Breakfast',
    Meal.lunch => 'Lunch',
    Meal.snacks => 'Snacks',
    Meal.dinner => 'Dinner',
  };
}

class MessItem {
  const MessItem(this.name, {this.nonVeg = false});

  final String name;
  final bool nonVeg;

  @override
  bool operator ==(Object other) =>
      other is MessItem && other.name == name && other.nonVeg == nonVeg;

  @override
  int get hashCode => Object.hash(name, nonVeg);

  @override
  String toString() => nonVeg ? '$name [non-veg]' : name;
}

class MessSheet {
  const MessSheet({
    required this.name,
    required this.month,
    required this.days,
    required this.blocks,
    required this.instructions,
  });

  final String name;

  /// 1–12 when the title names a month ("... FOR THE MONTH OF OCTOBER").
  final int? month;

  /// Raw cell text per day of month, per meal; date-specific variants are
  /// resolved by [itemsFor].
  final Map<int, Map<Meal, List<String>>> days;

  /// Every date sharing a day's block ("Thu 1, 15, 29").
  final Map<int, List<int>> blocks;
  final List<String> instructions;

  List<MessItem> itemsFor(int day, Meal meal) => [
    for (final raw in days[day]?[meal] ?? const <String>[])
      ?_resolve(raw, day, blocks[day] ?? const []),
  ];
}

class MessMenu {
  const MessMenu(this.sheets);

  final List<MessSheet> sheets;

  int? get month => sheets.map((s) => s.month).nonNulls.firstOrNull;
}

class MessMenuFormatException implements Exception {
  const MessMenuFormatException(this.message);
  final String message;

  @override
  String toString() => message;
}

const _months = [
  'january',
  'february',
  'march',
  'april',
  'may',
  'june',
  'july',
  'august',
  'september',
  'october',
  'november',
  'december',
];

final _dayCell = RegExp(
  r'^\s*(mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?[\s,:-]*((?:\d{1,2}[\s,&and]*)+)$',
  caseSensitive: false,
);

/// Parses decoded sheets (name → rows of cell text). Sheets that don't look
/// like a mess menu are skipped; throws when none do.
MessMenu parseMessMenu(Map<String, List<List<String>>> tables) {
  final sheets = <MessSheet>[];
  tables.forEach((name, rows) {
    final sheet = _parseSheet(name, rows);
    if (sheet != null) sheets.add(sheet);
  });
  if (sheets.isEmpty) {
    throw const MessMenuFormatException(
      "This spreadsheet doesn't look like a mess menu.",
    );
  }
  return MessMenu(sheets);
}

MessSheet? _parseSheet(String name, List<List<String>> rows) {
  int? month;
  int? headerRow;
  final mealColumns = <int, Meal>{};
  for (var r = 0; r < rows.length && headerRow == null; r++) {
    for (final cell in rows[r]) {
      final lower = cell.toLowerCase();
      if (month == null && lower.contains('menu')) {
        for (var m = 0; m < _months.length; m++) {
          if (lower.contains(_months[m])) month = m + 1;
        }
      }
    }
    final found = <int, Meal>{};
    for (var c = 0; c < rows[r].length; c++) {
      final lower = rows[r][c].trim().toLowerCase();
      for (final meal in Meal.values) {
        if (lower == meal.name || lower == meal.label.toLowerCase()) {
          found[c] = meal;
        }
      }
    }
    if (found.length >= 3) {
      headerRow = r;
      mealColumns.addAll(found);
    }
  }
  if (headerRow == null) return null;

  final days = <int, Map<Meal, List<String>>>{};
  final blocks = <int, List<int>>{};
  final instructions = <String>[];
  List<int>? current;
  var inInstructions = false;
  for (var r = headerRow + 1; r < rows.length; r++) {
    final row = rows[r];
    final joined = row.join(' ').toLowerCase();
    if (joined.contains('instruction')) {
      inInstructions = true;
      continue;
    }
    if (inInstructions) {
      for (final cell in row) {
        for (final line in cell.split('\n')) {
          final text = line.trim().replaceFirst(RegExp(r'^\d+\.\s*'), '');
          if (text.isNotEmpty) instructions.add(text);
        }
      }
      continue;
    }
    for (final cell in row) {
      final match = _dayCell.firstMatch(cell.trim());
      if (match != null) {
        current = [
          for (final d in RegExp(r'\d{1,2}').allMatches(match.group(2)!))
            int.parse(d.group(0)!),
        ].where((d) => d >= 1 && d <= 31).toList();
        for (final day in current) {
          blocks[day] = current;
        }
        break;
      }
    }
    if (current == null) continue;
    mealColumns.forEach((column, meal) {
      if (column >= row.length) return;
      final text = row[column].trim();
      if (text.isEmpty) return;
      for (final day in current!) {
        ((days[day] ??= {})[meal] ??= []).add(text);
      }
    });
  }
  if (days.isEmpty) return null;
  return MessSheet(
    name: name.trim(),
    month: month,
    days: days,
    blocks: blocks,
    instructions: instructions,
  );
}

final _datesInParens = RegExp(r'\((\d{1,2}(?:\s*,\s*\d{1,2})*)\)');
final _nonVegTag = RegExp(
  r'\(\s*(?:non[\s-]*veg|nv)\s*\)',
  caseSensitive: false,
);

/// "Aloo Mutter Curry (1), Chickpea Masala (15, 29)" on the 15th →
/// "Chickpea Masala"; null when no variant falls on [day].
/// Numbers in brackets only count as dates when they all belong to the
/// day's [block], so "(2)" as a quantity is left alone.
MessItem? _resolve(String raw, int day, List<int> block) {
  var text = raw.replaceAll(RegExp(r'\s+'), ' ').trim();
  final nonVeg = _nonVegTag.hasMatch(text);
  text = text.replaceAll(_nonVegTag, '').trim();

  final matches = _datesInParens.allMatches(text).toList();
  final variants =
      matches.isNotEmpty &&
      matches.every(
        (m) => RegExp(r'\d+')
            .allMatches(m.group(1)!)
            .every((d) => block.contains(int.parse(d.group(0)!))),
      );
  if (variants) {
    final picked = <String>[];
    var start = 0;
    for (final match in matches) {
      final dates = RegExp(
        r'\d+',
      ).allMatches(match.group(1)!).map((m) => int.parse(m.group(0)!));
      final label = text
          .substring(start, match.start)
          .replaceFirst(RegExp(r'^[\s,&/]*(and\s+)?', caseSensitive: false), '')
          .trim();
      if (dates.contains(day) && label.isNotEmpty) picked.add(label);
      start = match.end;
    }
    if (picked.isEmpty) return null;
    final tail = text.substring(start).trim();
    text = [picked.join(', '), if (tail.isNotEmpty) tail].join(' ');
  }
  text = text.replaceAll(RegExp(r'\s+'), ' ').trim();
  if (text.isEmpty) return null;
  return MessItem(text, nonVeg: nonVeg);
}

typedef MealWindow = ({int start, int end});
typedef MealWindows = Map<Meal, MealWindow>;

/// VIT-AP mess serving hours, in minutes since midnight; editable in
/// Settings.
const defaultMealWindows = <Meal, MealWindow>{
  Meal.breakfast: (start: 7 * 60 + 15, end: 9 * 60),
  Meal.lunch: (start: 12 * 60 + 30, end: 14 * 60),
  Meal.snacks: (start: 16 * 60 + 30, end: 18 * 60 + 15),
  Meal.dinner: (start: 19 * 60 + 15, end: 21 * 60),
};

/// "435-540,750-840,990-1095,1155-1260", breakfast to dinner.
String encodeMealWindows(MealWindows windows) => [
  for (final meal in Meal.values)
    '${windows[meal]!.start}-${windows[meal]!.end}',
].join(',');

/// Falls back to [defaultMealWindows] for anything missing or malformed.
MealWindows decodeMealWindows(String? raw) {
  final parts = raw?.split(',') ?? const <String>[];
  return {
    for (final meal in Meal.values)
      meal: () {
        if (meal.index >= parts.length) return defaultMealWindows[meal]!;
        final bounds = parts[meal.index].split('-').map(int.tryParse).toList();
        if (bounds.length != 2 || bounds.contains(null)) {
          return defaultMealWindows[meal]!;
        }
        return (start: bounds[0]!, end: bounds[1]!);
      }(),
  };
}

/// "7:15–9:00 AM", or "11:30 AM–1:00 PM" when the range crosses noon.
String mealWindowLabel(MealWindow window) {
  String clock(int minutes, String pattern) => DateFormat(
    pattern,
  ).format(DateTime(2000, 1, 1, minutes ~/ 60, minutes % 60));
  final samePeriod = (window.start < 12 * 60) == (window.end < 12 * 60);
  return samePeriod
      ? '${clock(window.start, 'h:mm')}–${clock(window.end, 'h:mm a')}'
      : '${clock(window.start, 'h:mm a')}–${clock(window.end, 'h:mm a')}';
}

/// The meal being served at [minute] of the day, else the next one; null
/// after dinner.
Meal? currentOrNextMeal(
  int minute, [
  MealWindows windows = defaultMealWindows,
]) {
  for (final meal in Meal.values) {
    if (minute < windows[meal]!.end) return meal;
  }
  return null;
}
