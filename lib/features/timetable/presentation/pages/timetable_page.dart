// lib/features/timetable/presentation/pages/timetable_page.dart
import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/timetable_colors.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/days_stack.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/timetable_card.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetablePage extends HookConsumerWidget {
  const TimetablePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = useMemoized(() => GlobalKey());
    final selectedDay = useState<int>(DateTime.now().weekday);
    final finalDay = useState<List<int>>([]);
    final timetableData = ref.watch(timetableProvider);
    final startX = useState<double?>(null);
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref.read(timetableProvider.notifier).updateTimetable();
        } catch (e, _) {
          log("$e");
        }
      });

      return null;
    }, []);

    Future<void> update() async {
      try {
        await ref.read(timetableProvider.notifier).updateTimetable();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    return Container(
      decoration: BoxDecoration(),
      child: Column(
        children: [
          if (timetableData.hasValue)
            DaysStack(
              selectedDay: selectedDay,
              daysList: getDayList(timetableData.value),
            ),

          Expanded(
            child: RefreshIndicator(
              key: key,
              backgroundColor: Colors.white,
              color: context.theme.colors.primary,
              strokeWidth: 2.5,
              onRefresh: update,
              child: SingleChildScrollView(
                physics: const AlwaysScrollableScrollPhysics(),
                child: GestureDetector(
                  onHorizontalDragStart: (details) {
                    startX.value = details.globalPosition.dx;
                  },
                  onHorizontalDragUpdate: (details) {
                    final currentX = details.globalPosition.dx;
                    final deltaX = currentX - (startX.value ?? currentX);

                    if (deltaX > 80 &&
                        finalDay.value.first < selectedDay.value) {
                      selectedDay.value -= 1;
                      startX.value = currentX;
                    } else if (deltaX < -80 &&
                        finalDay.value.last > selectedDay.value) {
                      selectedDay.value += 1;
                      startX.value = currentX;
                    }
                  },
                  onHorizontalDragEnd: (_) => startX.value = null,
                  child: ConstrainedBox(
                    constraints: BoxConstraints(
                      minHeight: MediaQuery.of(context).size.height * 0.8,
                    ),
                    child: timetableData.when(
                      data: (data) {
                        final tempList = getDayList(data);
                        finalDay.value = tempList;

                        if (!tempList.contains(selectedDay.value)) {
                          selectedDay.value = tempList.first;
                        }

                        final daySlots = addFreeSlots(
                          getDaySlotList(data, selectedDay.value),
                        );

                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            const SizedBox(height: 8),

                            ...daySlots.map(
                              (slot) => TimetableCard(slot: slot),
                            ),

                            Container(
                              margin: const EdgeInsets.all(16),
                              padding: const EdgeInsets.all(12),
                              decoration: BoxDecoration(
                                color: context.theme.colors.primaryForeground,
                                borderRadius: BorderRadius.circular(8),
                              ),
                              child: Text(
                                "Last updated: ${formatUnixTimestamp(data.updateTime.toInt())}",
                                textAlign: TextAlign.center,
                                style: TextStyle(
                                  fontSize: 12,
                                  color: context.theme.colors.primary,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ),
                          ],
                        );
                      },
                      error: (e, stackTrace) {
                        disCommonToast(context, e);
                        return Center(
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Icon(
                                Icons.error_outline,
                                size: 48,
                                color: Colors.grey[400],
                              ),
                              const SizedBox(height: 16),
                              Text(
                                commonErrorMessage(e),
                                textAlign: TextAlign.center,
                                style: TextStyle(
                                  fontSize: 16,
                                  color: Colors.grey[600],
                                ),
                              ),
                            ],
                          ),
                        );
                      },
                      loading:
                          () => const Center(
                            child: Column(
                              mainAxisAlignment: MainAxisAlignment.center,
                              children: [
                                SizedBox(
                                  width: 40,
                                  height: 40,
                                  child: CircularProgressIndicator(
                                    strokeWidth: 3,
                                    valueColor: AlwaysStoppedAnimation<Color>(
                                      TimetableColors.upcomingBorder,
                                    ),
                                  ),
                                ),
                                SizedBox(height: 16),
                                Text(
                                  'Loading timetable...',
                                  style: TextStyle(
                                    fontSize: 14,
                                    color: Colors.grey,
                                    fontWeight: FontWeight.w500,
                                  ),
                                ),
                              ],
                            ),
                          ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// Keep your existing helper functions unchanged
List<int> getDayList(TimetableData? data) {
  if (data == null) return [];
  Map<String, int> map = {
    "MON": 1,
    "TUE": 2,
    "WED": 3,
    "THU": 4,
    "FRI": 5,
    "SAT": 6,
    "SUN": 7,
  };
  Set found = {};
  for (final i in data.slots) {
    if (!found.contains(i.day)) {
      found.add(i.day);
    }
  }
  var out = found.map((k) => map[k]!).toList();
  out.sort();
  return out;
}

List<TimetableSlot> getDaySlotList(TimetableData data, int i) {
  Map<int, String> map = {
    1: "MON",
    2: "TUE",
    3: "WED",
    4: "THU",
    5: "FRI",
    6: "SAT",
    7: "SUN",
  };
  String day = map[i]!;
  List<TimetableSlot> slots = [];
  for (final slot in data.slots) {
    if (slot.day == day) {
      slots.add(slot);
    }
  }
  return slots;
}

List<TimetableSlot> addFreeSlots(List<TimetableSlot> t) {
  if (t.isEmpty) return t;

  List<TimetableSlot> r = [];
  for (int i = 0; i < t.length - 1; i++) {
    r.add(t[i]);
    final cClass = t[i].endTime;
    final nClass = t[i + 1].startTime;
    int diff = getdiff(cClass, nClass);
    int mod = (diff / 60).toInt();

    if (mod > 0) {
      r.add(
        TimetableSlot(
          serial: "-1",
          day: "-",
          slot: "$mod",
          courseCode: "-",
          courseType: "-",
          roomNo: "-",
          block: "-",
          startTime: cClass,
          endTime: nClass,
          name: "-",
        ),
      );
    }
  }
  r.add(t[t.length - 1]);
  return r;
}

int getdiff(String a, String b) {
  var first = a.split(":");
  var second = b.split(":");
  return (int.parse(second[0]) * 60 + int.parse(second[1])) -
      (int.parse(first[0]) * 60 + int.parse(first[1]));
}
