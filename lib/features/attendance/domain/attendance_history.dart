import 'package:intl/intl.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum ClassStatus { present, onDuty, absent, other }

ClassStatus classStatusOf(String raw) {
  final s = raw.toLowerCase().replaceAll(' ', '');
  if (s == 'present') return ClassStatus.present;
  if (s == 'onduty') return ClassStatus.onDuty;
  if (s == 'absent') return ClassStatus.absent;
  return ClassStatus.other;
}

/// Parses a history row's "dd-MM-yyyy" date.
DateTime? parseHistoryDate(String value) {
  try {
    return DateFormat('dd-MM-yyyy').parseStrict(value);
  } catch (_) {
    return null;
  }
}

/// (attended, total) rows in [data]; on duty counts as attended. Labs have
/// one row per two-period session.
(int, int) historyCount(FullAttendanceData data) {
  var attended = 0;
  var total = 0;
  for (final r in data.records) {
    switch (classStatusOf(r.status)) {
      case ClassStatus.present || ClassStatus.onDuty:
        attended++;
        total++;
      case ClassStatus.absent:
        total++;
      case ClassStatus.other:
        break;
    }
  }
  return (attended, total);
}

/// [data] without its latest row, as if VTOP had not posted that class to
/// the history yet. For testing the out-of-sync indicator.
FullAttendanceData withoutLatestEntry(FullAttendanceData data) {
  DateTime? latest;
  int? index;
  for (final (i, r) in data.records.indexed) {
    final date = parseHistoryDate(r.date);
    if (date != null && (latest == null || date.isAfter(latest))) {
      latest = date;
      index = i;
    }
  }
  if (index == null) return data;
  return data.copyWith(records: [...data.records]..removeAt(index));
}

/// How a course's class-by-class history lines up with its summary counts.
/// VTOP updates the two separately (and each is cached on its own), so they
/// can disagree for a while.
class HistorySync {
  const HistorySync._({
    required this.inSync,
    required this.attended,
    required this.total,
    required this.lastRecorded,
  });

  /// Compares [history] with [record]'s summary. Lab summaries count each
  /// session twice, so they are compared against double the history.
  factory HistorySync.of(AttendanceRecord record, FullAttendanceData history) {
    final (attended, total) = historyCount(history);
    final factor = record.islab() ? 2 : 1;
    final dates = history.records
        .map((r) => parseHistoryDate(r.date))
        .whereType<DateTime>();
    return HistorySync._(
      inSync:
          int.tryParse(record.classesAttended.trim()) == attended * factor &&
          int.tryParse(record.totalClasses.trim()) == total * factor,
      attended: attended,
      total: total,
      lastRecorded: dates.isEmpty
          ? null
          : dates.reduce((a, b) => a.isAfter(b) ? a : b),
    );
  }

  /// Whether the summary counts match the history.
  final bool inSync;

  /// The history's counts, in sessions for labs.
  final int attended;
  final int total;

  /// The latest class the history records.
  final DateTime? lastRecorded;

  /// The day the summary's counts run up to, when that is known: the last
  /// recorded class, if the history agrees with the summary.
  DateTime? get countedThrough => inSync ? lastRecorded : null;
}
