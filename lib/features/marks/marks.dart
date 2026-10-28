/// Marks for the selected semester.
library;

import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

export 'package:vitapmate/features/marks/logic/weightage_totals.dart';

final marksProvider = vtopPage<MarksData>(VtopQueries.marks);
