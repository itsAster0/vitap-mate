import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetableCard extends HookWidget {
  final TimetableSlot slot;
  const TimetableCard({super.key, required this.slot});

  @override
  Widget build(BuildContext context) {
    bool isnexthour(String startTimeStr, String endTimeStr) {
      final now = DateTime.now();
      final nextHour = now.add(Duration(minutes: 50));

      Duration parseTime(String timeStr) {
        final parts = timeStr.split(':').map(int.parse).toList();
        return Duration(hours: parts[0], minutes: parts[1]);
      }

      final nextHourDuration = Duration(
        hours: nextHour.hour,
        minutes: nextHour.minute,
      );
      final start = parseTime(startTimeStr);
      final end = parseTime(endTimeStr);

      return nextHourDuration >= start && nextHourDuration <= end;
    }

    return Padding(
      padding: const EdgeInsets.all(4.0),
      child: FFocusedOutline(
        focused: isnexthour(slot.startTime, slot.endTime),
        child: Container(
          decoration: BoxDecoration(
            color:
                slot.serial == "-1"
                    ? Color(0xFFFFE8CD)
                    : slot.courseType.endsWith("LA")
                    ? Color(0xFFD4F6FF)
                    : Color(0xFFC6E7FF),
            borderRadius: BorderRadius.circular(6),

            boxShadow: [
              BoxShadow(
                color: Colors.black12,
                blurRadius: 10,
                offset: Offset(0, 4),
              ),
            ],
          ),
          padding: const EdgeInsets.all(8),
          child:
              slot.serial != "-1"
                  ? Column(
                    spacing: 2,
                    children: [
                      Row(
                        spacing: 4,
                        children: [
                          Icon(
                            slot.courseType.endsWith("LA")
                                ? FIcons.flaskConical
                                : FIcons.libraryBig,

                            blendMode: BlendMode.darken,
                          ),
                          SizedBox(
                            width: MediaQuery.of(context).size.width * 0.8,
                            child: Text(
                              slot.name,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              "${slot.block} - ${slot.roomNo}",
                              style: TextStyle(
                                fontWeight: FontWeight.w400,
                                fontSize: 16,
                              ),
                            ),
                          ),
                          Text(slot.courseCode),
                        ],
                      ),
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              "${to12H(slot.startTime)} - ${to12H(slot.endTime)}",
                              style: TextStyle(
                                fontWeight: FontWeight.w400,
                                fontSize: 16,
                              ),
                            ),
                          ),
                          Text(slot.slot),
                        ],
                      ),
                    ],
                  )
                  : Column(
                    spacing: 2,
                    children: [
                      Row(
                        spacing: 4,
                        children: [
                          Icon(FIcons.hourglass),
                          Text("Free Time", overflow: TextOverflow.ellipsis),
                        ],
                      ),
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              "${to12H(slot.startTime)} - ${to12H(slot.endTime)}",
                              style: TextStyle(
                                fontWeight: FontWeight.w400,
                                fontSize: 18,
                              ),
                            ),
                          ),
                          Text(
                            slot.slot == "1"
                                ? "Slot ${slot.slot}"
                                : "Slots ${slot.slot}",
                          ),
                        ],
                      ),
                    ],
                  ),
        ),
      ),
    );
  }
}

String to12H(String time) {
  var list = time.split(":");
  int hours = int.parse(list[0]);
  if (hours > 12) {
    hours -= 12;
    list.add("PM");
  } else if (hours == 12) {
    list.add("PM");
  } else {
    list.add("AM");
  }
  return "$hours:${list[1]} ${list[2]} ";
}
