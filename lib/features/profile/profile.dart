/// The student's profile and their mentor. Not per semester.
library;

import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

final profileProvider = vtopGlobalPage<StudentProfile>(VtopQueries.profile);

final mentorProvider = vtopGlobalPage<MentorData>(VtopQueries.mentor);
