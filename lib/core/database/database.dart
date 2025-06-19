import 'package:drift/drift.dart';
import 'package:vitapmate/core/database/tables/tables.dart';
import 'package:drift_flutter/drift_flutter.dart';
// ignore: depend_on_referenced_packages
import 'package:path_provider/path_provider.dart';
part 'database.g.dart';

@DriftDatabase(
  tables: [
    SemesterTable,
    TimetableTable,
    FullAttendanceTable,
    AttendanceTable,
    ExamScheduleTable,
    MarksTable,
  ],
)
class AppDatabase extends _$AppDatabase {
  AppDatabase({QueryExecutor? executor, required String? username})
    : super(executor ?? _openConnection(username));

  @override
  int get schemaVersion => 1;

  static QueryExecutor _openConnection(String? name) {
    return driftDatabase(
      name: name ?? "NO_USERNAME",
      native: const DriftNativeOptions(
        databaseDirectory: getApplicationSupportDirectory,
      ),
    );
  }
}
