import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/domain/mess_menu.dart';
import 'package:vitapmate/features/docs/presentation/providers/docs_provider.dart';
import 'package:vitapmate/features/docs/presentation/providers/mess_timings_provider.dart';

const _sheetPrefKey = 'mess_menu_sheet';

/// A spreadsheet read as the monthly mess menu: opens on today, with the
/// meal being served (or up next) highlighted.
class MessMenuView extends HookConsumerWidget {
  const MessMenuView({super.key, required this.doc, this.compact = false});

  final DocWindow doc;

  /// Docs' "Continue viewing" card: only the meal on now or up next.
  final bool compact;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final attempt = useState(0);
    final menu = useFuture(
      useMemoized(() async {
        final repo = await ref.read(docsRepositoryProvider.future);
        return loadMessMenu(repo, doc);
      }, [doc.fileName, attempt.value]),
    );

    if (menu.connectionState != ConnectionState.done) {
      return Center(
        child: CircularProgressIndicator(
          color: context.theme.colors.primary,
          strokeWidth: 3,
        ),
      );
    }
    final data = menu.data;
    if (data == null) {
      return Center(
        child: EmptyState(
          icon: FLucideIcons.utensilsCrossed,
          title: 'Unable to read the mess menu',
          message:
              '${menu.error ?? 'The file is missing.'}\n'
              'Long-press it in Docs and choose "Show as spreadsheet" to see '
              'the file as it is.',
          action: FButton(
            variant: FButtonVariant.outline,
            onPress: () => attempt.value++,
            child: const Text('Try again'),
          ),
        ),
      );
    }
    final year = _yearFor(data.month, doc.name);
    final windows = ref.watch(messTimingsProvider);
    if (compact) return _UpNext(menu: data, year: year, windows: windows);
    return _MessMenuBody(menu: data, year: year, windows: windows);
  }
}

/// The menu only names the month; take the year from the document name
/// ("… October 2026") or the nearest one to today.
int _yearFor(int? month, String name) {
  final named = RegExp(r'\b(20\d\d)\b').firstMatch(name);
  if (named != null) return int.parse(named.group(1)!);
  final now = DateTime.now();
  if (month == null) return now.year;
  if (month - now.month > 6) return now.year - 1;
  if (now.month - month > 6) return now.year + 1;
  return now.year;
}

/// The Regular/Special choice, remembered across opens.
ValueNotifier<int> _useSheetChoice(MessMenu menu) {
  final sheetIndex = useState(0);
  useEffect(() {
    var cancelled = false;
    SharedPreferences.getInstance().then((prefs) {
      final saved = prefs.getString(_sheetPrefKey);
      final index = menu.sheets.indexWhere((s) => s.name == saved);
      if (!cancelled && index != -1) sheetIndex.value = index;
    });
    return () => cancelled = true;
  }, const []);
  return sheetIndex;
}

MessSheet _sheetAt(MessMenu menu, int index) =>
    menu.sheets[index.clamp(0, menu.sheets.length - 1)];

class _UpNext extends HookWidget {
  const _UpNext({
    required this.menu,
    required this.year,
    required this.windows,
  });

  final MessMenu menu;
  final int year;
  final MealWindows windows;

  @override
  Widget build(BuildContext context) {
    final sheet = _sheetAt(menu, _useSheetChoice(menu).value);
    final now = DateTime.now();
    final minute = now.hour * 60 + now.minute;
    var meal = currentOrNextMeal(minute, windows);
    var day = DateTime(now.year, now.month, now.day);
    // After dinner the next meal is tomorrow's breakfast.
    if (meal == null) {
      meal = Meal.breakfast;
      day = day.add(const Duration(days: 1));
    }
    final colors = context.theme.colors;
    final inMenu =
        day.year == year &&
        day.month == (menu.month ?? day.month) &&
        sheet.days.containsKey(day.day);
    if (!inMenu) {
      return Center(
        child: Text(
          'No menu for ${DateFormat('d MMMM').format(day)} in this file',
          style: context.theme.typography.body.sm.copyWith(
            color: colors.mutedForeground,
          ),
        ),
      );
    }
    final today = day.day == now.day;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(Space.md + 2),
      child: _MealCard(
        meal: meal,
        items: sheet.itemsFor(day.day, meal),
        window: windows[meal]!,
        status: today && minute >= windows[meal]!.start
            ? _MealStatus.now
            : _MealStatus.next,
        when: today ? null : 'Tomorrow',
        framed: false,
      ),
    );
  }
}

class _MessMenuBody extends HookWidget {
  const _MessMenuBody({
    required this.menu,
    required this.year,
    required this.windows,
  });

  final MessMenu menu;
  final int year;
  final MealWindows windows;

  @override
  Widget build(BuildContext context) {
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final month = menu.month ?? now.month;
    final first = DateTime(year, month);
    final last = DateTime(year, month + 1, 0);
    final inMonth = !today.isBefore(first) && !today.isAfter(last);

    final selected = useState(inMonth ? today : first);
    final sheetIndex = _useSheetChoice(menu);
    final showRules = useState(false);

    void selectSheet(int index) {
      sheetIndex.value = index;
      unawaited(
        SharedPreferences.getInstance().then(
          (prefs) => prefs.setString(_sheetPrefKey, menu.sheets[index].name),
        ),
      );
    }

    final sheet = _sheetAt(menu, sheetIndex.value);
    final day = selected.value;
    final isToday = day == today;
    final minute = now.hour * 60 + now.minute;
    final focus = isToday ? currentOrNextMeal(minute, windows) : null;
    final hasDay = sheet.days.containsKey(day.day);

    final colors = context.theme.colors;
    final typography = context.theme.typography;

    return ListView(
      padding: const EdgeInsets.fromLTRB(Space.md, Space.sm, Space.md, 96),
      children: [
        if (menu.sheets.length > 1) ...[
          Segmented<int>(
            value: sheetIndex.value,
            onChanged: selectSheet,
            segments: [
              for (var i = 0; i < menu.sheets.length; i++)
                (i, menu.sheets[i].name),
            ],
          ),
          const SizedBox(height: Space.md),
        ],
        _WeekStrip(first: first, last: last, today: today, selected: selected),
        const SizedBox(height: Space.md),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 2),
          child: Text(
            isToday
                ? 'Today · ${DateFormat('EEEE, d MMMM').format(day)}'
                : DateFormat('EEEE, d MMMM').format(day),
            style: typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
              color: colors.mutedForeground,
            ),
          ),
        ),
        const SizedBox(height: Space.sm),
        if (!hasDay)
          const EmptyState(
            icon: FLucideIcons.utensilsCrossed,
            title: 'No menu for this day',
          )
        else
          for (final meal in Meal.values) ...[
            _MealCard(
              meal: meal,
              items: sheet.itemsFor(day.day, meal),
              window: windows[meal]!,
              status: focus == null
                  ? (isToday ? _MealStatus.past : _MealStatus.normal)
                  : meal == focus
                  ? (minute >= windows[meal]!.start
                        ? _MealStatus.now
                        : _MealStatus.next)
                  : meal.index < focus.index
                  ? _MealStatus.past
                  : _MealStatus.normal,
            ),
            const SizedBox(height: Space.sm + 2),
          ],
        if (sheet.instructions.isNotEmpty) ...[
          const SizedBox(height: Space.xs),
          FTileGroup(
            children: [
              FTile(
                prefix: const Icon(FLucideIcons.clipboardList),
                title: Text('Mess instructions (${sheet.instructions.length})'),
                suffix: Icon(
                  showRules.value
                      ? FLucideIcons.chevronUp
                      : FLucideIcons.chevronDown,
                ),
                onPress: () => showRules.value = !showRules.value,
              ),
            ],
          ),
          if (showRules.value)
            Padding(
              padding: const EdgeInsets.fromLTRB(
                Space.sm,
                Space.sm,
                Space.sm,
                0,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final (i, rule) in sheet.instructions.indexed)
                    Padding(
                      padding: const EdgeInsets.only(bottom: Space.sm),
                      child: Text(
                        '${i + 1}. $rule',
                        style: typography.body.xs.copyWith(
                          height: 1.4,
                          color: colors.mutedForeground,
                        ),
                      ),
                    ),
                ],
              ),
            ),
        ],
      ],
    );
  }
}

/// Mon–Sun of the selected week, with arrows to step through the month.
class _WeekStrip extends StatelessWidget {
  const _WeekStrip({
    required this.first,
    required this.last,
    required this.today,
    required this.selected,
  });

  final DateTime first;
  final DateTime last;
  final DateTime today;
  final ValueNotifier<DateTime> selected;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final day = selected.value;
    final monday = day.subtract(Duration(days: day.weekday - 1));
    final week = [for (var i = 0; i < 7; i++) monday.add(Duration(days: i))];
    final canBack = monday.isAfter(first);
    final canForward = week.last.isBefore(last);

    void step(int weeks) {
      var next = day.add(Duration(days: 7 * weeks));
      if (next.isBefore(first)) next = first;
      if (next.isAfter(last)) next = last;
      selected.value = next;
    }

    Widget arrow(IconData icon, bool enabled, VoidCallback onPress) =>
        PressScale(
          scale: 0.9,
          semanticsLabel: icon == FLucideIcons.chevronLeft
              ? 'Previous week'
              : 'Next week',
          onPress: enabled ? onPress : () {},
          child: SizedBox(
            width: 24,
            height: 54,
            child: Icon(
              icon,
              size: 18,
              color: enabled
                  ? colors.mutedForeground
                  : colors.mutedForeground.withValues(alpha: 0.25),
            ),
          ),
        );

    return Row(
      children: [
        arrow(FLucideIcons.chevronLeft, canBack, () => step(-1)),
        for (final date in week)
          Expanded(
            child: Builder(
              builder: (context) {
                final outside = date.isBefore(first) || date.isAfter(last);
                final isSelected = date == day;
                final isToday = date == today;
                return PressScale(
                  scale: 0.94,
                  semanticsLabel: DateFormat('EEEE, d MMMM').format(date),
                  onPress: outside ? () {} : () => selected.value = date,
                  child: AnimatedContainer(
                    duration: Motion.medium,
                    curve: Curves.easeOutCubic,
                    height: 54,
                    margin: const EdgeInsets.symmetric(horizontal: 2),
                    decoration: BoxDecoration(
                      color: isSelected
                          ? colors.primary
                          : const Color(0x00000000),
                      borderRadius: BorderRadius.circular(Radii.md),
                      border: Border.all(
                        color: isSelected
                            ? colors.primary
                            : isToday
                            ? colors.app.accent.withValues(alpha: 0.5)
                            : const Color(0x00000000),
                      ),
                    ),
                    child: Opacity(
                      opacity: outside ? 0.25 : 1,
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Text(
                            DateFormat('EEE').format(date).toUpperCase(),
                            style: TextStyle(
                              fontSize: 10,
                              letterSpacing: 0.6,
                              fontWeight: FontWeight.w600,
                              color: isSelected
                                  ? colors.primaryForeground.withValues(
                                      alpha: 0.7,
                                    )
                                  : colors.mutedForeground,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            '${date.day}',
                            style: TextStyle(
                              fontSize: 17,
                              fontWeight: FontWeight.w700,
                              color: isSelected
                                  ? colors.primaryForeground
                                  : isToday
                                  ? colors.app.accent
                                  : colors.foreground,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
        arrow(FLucideIcons.chevronRight, canForward, () => step(1)),
      ],
    );
  }
}

enum _MealStatus { normal, past, now, next }

class _MealCard extends StatelessWidget {
  const _MealCard({
    required this.meal,
    required this.items,
    required this.status,
    required this.window,
    this.when,
    this.framed = true,
  });

  final Meal meal;
  final List<MessItem> items;
  final _MealStatus status;
  final MealWindow window;

  /// Shown before the meal name, e.g. "Tomorrow".
  final String? when;

  /// Off inside a card that already has its own border.
  final bool framed;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final range = mealWindowLabel(window);
    final live = status == _MealStatus.now || status == _MealStatus.next;

    final body = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(
              [?when, meal.label].join(' · ').toUpperCase(),
              style: typography.body.xs.copyWith(
                fontWeight: FontWeight.w700,
                letterSpacing: 0.8,
                color: live
                    ? colors.app.accentTone.onSubtle
                    : colors.foreground,
              ),
            ),
            const SizedBox(width: Space.sm),
            Expanded(
              child: Text(
                range,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: typography.body.xs.copyWith(
                  color: colors.mutedForeground,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ),
            if (status == _MealStatus.now)
              ToneBadge(label: 'NOW', tone: colors.app.accentTone, solid: true)
            else if (status == _MealStatus.next)
              ToneBadge(label: 'NEXT', tone: colors.app.accentTone),
          ],
        ),
        const SizedBox(height: Space.sm + 2),
        if (items.isEmpty)
          Text(
            'Nothing listed',
            style: typography.body.sm.copyWith(color: colors.mutedForeground),
          )
        else
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: [for (final item in items) _ItemPill(item: item)],
          ),
      ],
    );
    if (!framed) return body;
    return AnimatedOpacity(
      duration: Motion.medium,
      opacity: status == _MealStatus.past ? 0.5 : 1,
      child: Surface(
        padding: const EdgeInsets.all(Space.md + 2),
        borderColor: live ? colors.app.accent.withValues(alpha: 0.45) : null,
        child: body,
      ),
    );
  }
}

class _ItemPill extends StatelessWidget {
  const _ItemPill({required this.item});

  final MessItem item;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final danger = colors.app.danger;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: item.nonVeg ? danger.subtle : colors.secondary,
        borderRadius: BorderRadius.circular(Radii.sm + 2),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (item.nonVeg) ...[
            Container(
              width: 6,
              height: 6,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: danger.base,
              ),
            ),
            const SizedBox(width: 6),
          ],
          Flexible(
            child: Text(
              item.name,
              style: context.theme.typography.body.xs.copyWith(
                fontWeight: FontWeight.w500,
                color: item.nonVeg ? danger.onSubtle : colors.foreground,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
