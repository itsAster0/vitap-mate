import 'dart:convert';

import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradeHistoryModel {
  static GradeHistoryData fromLocal(GradeHistoryCacheTableData row) {
    final decoded = jsonDecode(row.payload) as Map<String, dynamic>;
    final parsed = GradeHistoryData.fromJson(decoded);
    return parsed.copyWith(updateTime: BigInt.from(row.time));
  }
}
