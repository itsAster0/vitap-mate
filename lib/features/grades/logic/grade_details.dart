import 'package:vitapmate/features/marks/marks.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// "4" for whole numbers, else one decimal: "2.5".
String fmtNum(double v) =>
    v == v.roundToDouble() ? v.toStringAsFixed(0) : v.toStringAsFixed(1);

/// ">=83 and <88" → "83–88", ">=88" → "≥88", "<50" → "<50".
String shortRange(String raw) {
  final r = raw.replaceAll('#', '').trim();
  final nums = [for (final m in RegExp(r'\d+(?:\.\d+)?').allMatches(r)) m[0]!];
  return switch (nums) {
    [final lo, final hi, ...] => '$lo–$hi',
    [final n] => r.startsWith('<') ? '<$n' : '≥$n',
    _ => r,
  };
}

/// Marks grouped by the section before "•" in their title ("Theory •
/// CAT-1"), in first-seen order, each with its weighted total.
List<({String title, String total, List<GradeDetailMark> marks})> gradeSections(
  GradeDetailsData detail,
) {
  final grouped = <String, List<GradeDetailMark>>{};
  for (final mark in detail.marks) {
    final parts = mark.markTitle.split("•");
    final (section, title) = parts.length >= 2
        ? (parts.first.trim(), parts.skip(1).join("•").trim())
        : (detail.classCourseType, mark.markTitle);
    grouped.putIfAbsent(section, () => []).add(mark.copyWith(markTitle: title));
  }
  return [
    for (final MapEntry(key: title, value: marks) in grouped.entries)
      (title: title, total: _weightedTotal(marks), marks: marks),
  ];
}

String _weightedTotal(List<GradeDetailMark> marks) {
  final totals = calculateWeightageTotals<GradeDetailMark>(
    marks,
    titleOf: (m) => m.markTitle,
    gainedOf: (m) => m.weightageMark,
    possibleOf: (m) => m.weightage,
  );
  if (totals.gained == 0) return "";
  return totals.possible > 0
      ? '${fmtNum(totals.gained)} / ${fmtNum(totals.possible)}'
      : fmtNum(totals.gained);
}
