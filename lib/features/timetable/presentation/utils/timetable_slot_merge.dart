import 'package:vitapmate/src/api/vtop/types.dart';

List<TimetableSlot> mergeLabsSloths(List<TimetableSlot> input) {
  final t = [...input];
  t.sort((a, b) {
    final t1 = _parseTime(a.startTime);
    final t2 = _parseTime(b.startTime);
    return t1.compareTo(t2);
  });

  final r = <TimetableSlot>[];
  for (var i = 0; i < t.length; i++) {
    final current = t[i];
    final prev = r.isNotEmpty ? r.last : null;
    if (prev != null &&
        current.isLab &&
        prev.isLab &&
        current.courseCode == prev.courseCode) {
      r[r.length - 1] = prev.copyWith(
        endTime: current.endTime,
        slot: "${prev.slot}+${current.slot}",
      );
    } else {
      r.add(current);
    }
  }
  return r;
}

Duration _parseTime(String t) {
  final parts = t.split(":");
  final h = int.parse(parts[0]);
  final m = parts.length > 1 ? int.parse(parts[1]) : 0;
  return Duration(hours: h, minutes: m);
}
