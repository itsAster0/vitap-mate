import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';

List<DateTime> getCurrentWeekDates() {
  final now = DateTime.now();
  final referenceDate =
      now.weekday == DateTime.sunday ? now.add(const Duration(days: 1)) : now;
  final startOfWeek = referenceDate.subtract(
    Duration(days: referenceDate.weekday - 1),
  );

  return List.generate(7, (index) => startOfWeek.add(Duration(days: index)));
}

class DaysStack extends HookWidget {
  final ValueNotifier<int> selectedDay;
  final List<int> daysList;
  const DaysStack({
    super.key,
    required this.selectedDay,
    required this.daysList,
  });

  @override
  Widget build(BuildContext context) {
    final days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
    final dateList = getCurrentWeekDates();
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        spacing: 8,
        children: [
          for (final i in daysList)
            SizedBox(
              width: 50,
              height: 60,
              child: Stack(
                children: [
                  FTappable(
                    onPress: () {
                      selectedDay.value = i;
                    },
                    builder:
                        (context, data, child) => Container(
                          decoration: BoxDecoration(
                            color:
                                selectedDay.value == i
                                    ? context.theme.colors.primary
                                    : context.theme.colors.background,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(
                              color: context.theme.colors.border,
                            ),
                          ),
                          padding: const EdgeInsets.all(0),
                          child: child!,
                        ),
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      spacing: 0,
                      children: [
                        Center(
                          child: Text(
                            dateList[i - 1].day.toString(),
                            textHeightBehavior: TextHeightBehavior(
                              applyHeightToFirstAscent: false,
                              applyHeightToLastDescent: false,
                            ),
                            style: TextStyle(
                              fontWeight: FontWeight.bold,
                              color:
                                  selectedDay.value != i
                                      ? context.theme.colors.primary
                                      : context.theme.colors.background,
                            ),
                          ),
                        ),
                        Center(
                          child: Text(
                            days[i - 1],
                            textHeightBehavior: TextHeightBehavior(
                              applyHeightToFirstAscent: false,
                              applyHeightToLastDescent: false,
                            ),
                            style: TextStyle(
                              fontSize: 14,
                              color:
                                  selectedDay.value != i
                                      ? context.theme.colors.primary
                                      : context.theme.colors.background,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (DateTime.now().weekday == i)
                    Positioned(
                      top: 8,
                      right: 8,
                      child: CircleAvatar(
                        backgroundColor:
                            selectedDay.value == i
                                ? context.theme.colors.secondary
                                : context.theme.colors.primary,
                        //minRadius: 5,
                        radius: 4,
                      ),
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
