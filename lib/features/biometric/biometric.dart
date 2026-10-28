/// Face and biometric entries for one date (DD/MM/YYYY).
library;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

final biometricHistoryProvider =
    AsyncNotifierProvider.family<
      VtopPage<BiometricData>,
      BiometricData,
      String
    >(
      (date) =>
          VtopPage((_) => VtopQueries.biometric(date), perSemester: false),
    );
