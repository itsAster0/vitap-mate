/// The student's semesters, newest first. Not per semester.
library;

import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

final semesterIdProvider = vtopGlobalPage<SemesterData>(VtopQueries.semesters);
