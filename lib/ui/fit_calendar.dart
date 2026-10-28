import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// A day-grid [FCalendar] that fits the width it is given.
///
/// forui sizes the calendar from fixed day cells (7 × 44 on touch), so on
/// narrow phones it is wider than a dialog and Friday and Saturday get cut
/// off. This shrinks the cells to the space available (never past the
/// theme's size), keeping the text crisp rather than scaling it.
///
/// [boxed] keeps the calendar's own border and padding. Inside a dialog or
/// sheet that already frames it, leave it off to avoid a box in a box.
class FitCalendar extends StatelessWidget {
  const FitCalendar({
    super.key,
    required this.selectionControl,
    this.control = const FGridCalendarControl(),
    this.onDayPress,
    this.boxed = false,
  });

  final FDateSelectionControl selectionControl;
  final FGridCalendarControl control;
  final FutureOr<void> Function(DateTime)? onDayPress;
  final bool boxed;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme.calendarStyle;
    final padding = boxed
        ? theme.padding.resolve(Directionality.of(context)).horizontal
        : 0.0;
    final themeCell = theme.dayPickerStyle.daySize.width;
    return LayoutBuilder(
      builder: (context, constraints) {
        final available = constraints.maxWidth.isFinite
            ? constraints.maxWidth - padding
            : themeCell * DateTime.daysPerWeek;
        final cell = math
            .min(themeCell, available / DateTime.daysPerWeek)
            .floorToDouble();
        return FCalendar.grid(
          control: control,
          selectionControl: selectionControl,
          onDayPress: onDayPress,
          // The month title and arrows keep their size when the cells
          // shrink, so the header alone is laid out at its natural width
          // and scaled to the row.
          headerBuilder: (context, controller, selection, child) =>
              cell >= themeCell
              ? child
              : FittedBox(
                  fit: BoxFit.scaleDown,
                  child: SizedBox(
                    width: themeCell * DateTime.daysPerWeek,
                    child: child,
                  ),
                ),
          style: FCalendarStyleDelta.delta(
            dayPickerStyle: FCalendarDayPickerStyleDelta.delta(
              daySize: Size.square(cell),
            ),

            decoration: boxed
                ? null
                : const DecorationDelta.value(BoxDecoration()),
            padding: boxed
                ? null
                : const EdgeInsetsGeometryDelta.value(EdgeInsets.zero),
          ),
        );
      },
    );
  }
}
