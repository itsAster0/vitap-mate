import 'package:drift/drift.dart';
import 'package:vitapmate/core/database/tables/tables.dart';
import 'package:drift_flutter/drift_flutter.dart';
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
    GradeCourseTable,
    GradeDetailTable,
    GradeHistoryCacheTable,
  ],
)
class AppDatabase extends _$AppDatabase {
  AppDatabase({QueryExecutor? executor, required String? username})
    : super(executor ?? _openConnection(username));

  @override
  int get schemaVersion => 3;
  @override
  MigrationStrategy get migration {
    return MigrationStrategy(
      onCreate: (Migrator m) async {
        await m.createAll();
      },
      onUpgrade: (Migrator m, int from, int to) async {
        if (from < 2) {
          await m.addColumn(timetableTable, timetableTable.isLab);
          await m.addColumn(timetableTable, timetableTable.faculty);
        }
        if (from < 3) {
          await m.createTable(gradeCourseTable);
          await m.createTable(gradeDetailTable);
          await m.createTable(gradeHistoryCacheTable);
        }
      },
    );
  }

  static QueryExecutor _openConnection(String? name) {
    return driftDatabase(
      name: name ?? "NO_USERNAME",
      native: const DriftNativeOptions(
        databaseDirectory: getApplicationSupportDirectory,
      ),
    );
  }
}
