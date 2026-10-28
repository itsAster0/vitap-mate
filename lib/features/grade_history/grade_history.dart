/// Every graded course across semesters, with CGPA.
library;

import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

final gradeHistoryProvider = vtopGlobalPage<GradeHistoryData>(
  VtopQueries.gradeHistory,
);
