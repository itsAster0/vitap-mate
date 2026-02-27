import 'dart:convert';

import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesModel {
  static GradeViewData toViewFromLocal(List<GradeCourseTableData> rows) {
    if (rows.isEmpty) {
      return GradeViewData(
        courses: const [],
        semesters: const [],
        semesterId: "",
        updateTime: BigInt.zero,
      );
    }
    return GradeViewData(
      courses: [
        for (final r in rows)
          GradeCourseRecord(
            serial: "${r.serial}",
            courseCode: r.courseCode,
            courseTitle: r.courseTitle,
            courseType: r.courseType,
            gradingType: r.gradingType,
            grandTotal: r.grandTotal,
            grade: r.grade,
            courseId: r.courseId,
          ),
      ],
      semesters: const [],
      semesterId: rows.first.semId,
      updateTime: BigInt.from(rows.first.time),
    );
  }

  static Map<String, GradeDetailsData> toDetailsMapFromLocal(
    List<GradeDetailTableData> rows,
  ) {
    final grouped = <String, List<GradeDetailTableData>>{};
    for (final row in rows) {
      grouped.putIfAbsent(row.courseId, () => []).add(row);
    }

    final result = <String, GradeDetailsData>{};
    for (final entry in grouped.entries) {
      final group = entry.value;
      if (group.isEmpty) continue;
      final first = group.first;
      final marks = [
        for (final r in group)
          GradeDetailMark(
            serial: "${r.serial}",
            markTitle: r.markTitle,
            maxMark: r.maxMark,
            weightage: r.weightage,
            status: r.status,
            scoredMark: r.scoredMark,
            weightageMark: r.weightageMark,
          ),
      ];
      List<GradeRange> gradeRanges = const [];
      try {
        final decoded = jsonDecode(first.gradeRanges) as List<dynamic>;
        gradeRanges =
            decoded
                .map(
                  (e) =>
                      GradeRange.fromJson(Map<String, dynamic>.from(e as Map)),
                )
                .toList();
      } catch (_) {}
      result[entry.key] = GradeDetailsData(
        semesterId: first.semId,
        courseId: first.courseId,
        classNumber: first.classNumber,
        classCourseType: first.classCourseType,
        grandTotal: first.grandTotal,
        marks: marks,
        gradeRanges: gradeRanges,
        updateTime: BigInt.from(first.time),
      );
    }
    return result;
  }
}
