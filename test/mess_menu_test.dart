import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/docs/domain/mess_menu.dart';

List<String> row(List<String> cells) => cells;

final _sheet = [
  row(['', '', '', '', '']),
  row(['VEG & NON-VEG MESS MENU FOR THE MONTH OF OCTOBER', '', '', '', '']),
  row(['Day', 'Breakfast', 'Lunch', 'Snacks', 'Dinner']),
  row([
    'Thu\n1, 15, 29',
    'Methi Poori',
    'Salad',
    'Veg Puff (1, 29), Boiled Peanuts (15)',
    'Roti',
  ]),
  row([
    '',
    'Aloo Mutter Curry (1), Chickpea Masala (15, 29)',
    'Pulka',
    'Tea',
    '',
  ]),
  row(['', 'Masala Onion Omlet (Non-Veg)', 'Rice (2)', '', 'Fish Fry (NV)']),
  row(['Fri\n2, 16, 30', 'Uggani', '', '', 'Paratha']),
  row([
    '',
    'Onion Dosa (2), Masala Dosa (16, 30) - 2 Pcs',
    '',
    '',
    'Fruit Custard (2), and Custard Apple (16, 30)',
  ]),
  row(['', 'MESS SERVICE INSTRUCTIONS', '', '', '']),
  row(['', '1. Curd daily.\n2. Fresh salad.', '', '', '']),
];

void main() {
  final menu = parseMessMenu({
    'Veg & Non-Veg': _sheet,
    'Notes': [
      row(['hi']),
    ],
  });
  final sheet = menu.sheets.single;

  test('reads the month and every date of each block', () {
    expect(menu.month, 10);
    expect(sheet.days.keys, containsAll([1, 15, 29, 2, 16, 30]));
    expect(sheet.itemsFor(15, Meal.breakfast).first.name, 'Methi Poori');
  });

  test('keeps only the variant for the date', () {
    expect(sheet.itemsFor(15, Meal.snacks).map((i) => i.name), [
      'Boiled Peanuts',
      'Tea',
    ]);
    expect(sheet.itemsFor(1, Meal.breakfast)[1].name, 'Aloo Mutter Curry');
    expect(sheet.itemsFor(29, Meal.breakfast)[1].name, 'Chickpea Masala');
    expect(sheet.itemsFor(16, Meal.breakfast)[1].name, 'Masala Dosa - 2 Pcs');
    expect(sheet.itemsFor(30, Meal.dinner)[1].name, 'Custard Apple');
  });

  test('brackets that are not dates of the block stay as text', () {
    expect(sheet.itemsFor(1, Meal.lunch).last.name, 'Rice (2)');
  });

  test('tags non-veg items', () {
    final omelette = sheet.itemsFor(1, Meal.breakfast).last;
    expect(omelette, const MessItem('Masala Onion Omlet', nonVeg: true));
    expect(
      sheet.itemsFor(1, Meal.dinner).last,
      const MessItem('Fish Fry', nonVeg: true),
    );
  });

  test('collects instructions', () {
    expect(sheet.instructions, ['Curd daily.', 'Fresh salad.']);
    expect(sheet.itemsFor(2, Meal.breakfast).map((i) => i.name), [
      'Uggani',
      'Onion Dosa - 2 Pcs',
    ]);
  });

  test('rejects sheets that are not a menu', () {
    expect(
      () => parseMessMenu({
        'S': [
          row(['a', 'b']),
        ],
      }),
      throwsA(isA<MessMenuFormatException>()),
    );
  });

  test('current or next meal', () {
    expect(currentOrNextMeal(6 * 60), Meal.breakfast);
    expect(currentOrNextMeal(13 * 60), Meal.lunch);
    expect(currentOrNextMeal(14 * 60), Meal.snacks);
    expect(currentOrNextMeal(21 * 60), isNull);
  });

  test('timings round-trip and fall back per meal', () {
    final custom = {
      ...defaultMealWindows,
      Meal.lunch: (start: 12 * 60, end: 13 * 60),
    };
    expect(decodeMealWindows(encodeMealWindows(custom)), custom);
    expect(decodeMealWindows(null), defaultMealWindows);
    expect(
      decodeMealWindows('1-2,oops')[Meal.lunch],
      defaultMealWindows[Meal.lunch],
    );
    expect(decodeMealWindows('1-2,oops')[Meal.breakfast], (start: 1, end: 2));
    expect(currentOrNextMeal(13 * 60, custom), Meal.snacks);
  });

  test('time labels', () {
    expect(
      mealWindowLabel(defaultMealWindows[Meal.breakfast]!),
      '7:15–9:00 AM',
    );
    expect(mealWindowLabel((start: 690, end: 780)), '11:30 AM–1:00 PM');
  });
}
