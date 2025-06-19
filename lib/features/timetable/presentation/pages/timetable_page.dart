import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
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
    final controller = usePageController(initialPage: DateTime.now().weekday);
    final timetableData = ref.watch(timetableProvider);
    final startX = useState<double?>(null);

    Future<void> update() async {
      try {
        await ref.read(timetableProvider.notifier).updateTimetable();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    return Padding(
      padding: const EdgeInsets.all(0),
      child: Column(
        spacing: 2,
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
              color: Colors.black,
              onRefresh: () async {
                //await Future.delayed(Duration(seconds: 10));
                await update();
              },
              child: SingleChildScrollView(
                physics: const AlwaysScrollableScrollPhysics(),
                child: GestureDetector(
                  onHorizontalDragStart: (details) {
                    startX.value = details.globalPosition.dx;
                  },
                  onHorizontalDragUpdate: (details) {
                    final currentX = details.globalPosition.dx;
                    final deltaX = currentX - (startX.value ?? currentX);

                    if (deltaX > 80) {
                      if (finalDay.value.first < selectedDay.value) {
                        selectedDay.value -= 1;
                      }
                      startX.value = currentX;
                    } else if (deltaX < -80) {
                      if (finalDay.value.last > selectedDay.value) {
                        selectedDay.value += 1;
                      }
                      startX.value = currentX;
                    }
                  },
                  onHorizontalDragEnd: (_) {
                    startX.value = null;
                  },
                  child: ConstrainedBox(
                    constraints: BoxConstraints(
                      minHeight: MediaQuery.of(context).size.height * 0.8,
                    ),
                    child: timetableData.when(
                      data: (data) {
                        var tempList = getDayList(data);
                        finalDay.value = tempList;
                        if (!tempList.contains(selectedDay.value)) {
                          selectedDay.value = tempList.first;
                        }
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            for (final slot in addFreeSlots(
                              getDaySlotList(data, selectedDay.value),
                            ))
                              TimetableCard(slot: slot),
                            Center(
                              child: Padding(
                                padding: const EdgeInsets.only(
                                  top: 12,
                                  bottom: 20,
                                ),
                                child: Text(
                                  "Data updated on ${formatUnixTimestamp(data.updateTime.toInt())}",
                                  style: TextStyle(
                                    fontSize: 14,
                                    color: Colors.grey[600],
                                  ),
                                ),
                              ),
                            ),
                          ],
                        );
                      },
                      error: (e, er) {
                        disCommonToast(context, e);
                        String msg = commonErrorMessage(e);
                        return Center(child: Text(msg));
                      },
                      loading: () {
                        //print("loading");
                        return Center(
                          child: SizedBox(
                            width: 50,
                            height: 50,
                            child: CircularProgressIndicator(
                              color: Colors.black,
                            ),
                          ),
                        );
                      },
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
  for (final i in data.slots) {
    if (i.day == day) {
      slots.add(i);
    }
  }
  return slots;
}

List<TimetableSlot> addFreeSlots(List<TimetableSlot> t) {
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

getdiff(a, b) {
  var first = a.split(":");
  var second = b.split(":");
  return (int.parse(second[0]) * 60 + int.parse(second[1])) -
      (int.parse(first[0]) * 60 + int.parse(first[1]));
}
