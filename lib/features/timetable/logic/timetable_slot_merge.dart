import 'package:vitapmate/features/timetable/logic/time_format.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// [input] by start time, with back-to-back periods of the same lab joined
/// into one session ("L1+L2").
List<TimetableSlot> mergeLabsSloths(List<TimetableSlot> input) {
  final merged = <TimetableSlot>[];
  for (final slot in [...input]..sort(byStart)) {
    final prev = merged.lastOrNull;
    if (prev != null &&
        slot.kind == ClassKind.lab &&
        prev.kind == ClassKind.lab &&
        slot.courseCode == prev.courseCode) {
      merged.last = prev.copyWith(
        endTime: slot.endTime,
        slot: '${prev.slot}+${slot.slot}',
      );
    } else {
      merged.add(slot);
    }
  }
  return merged;
}

/// Orders slots by start time.
int byStart(TimetableSlot a, TimetableSlot b) =>
    minutesOf(a.startTime).compareTo(minutesOf(b.startTime));
