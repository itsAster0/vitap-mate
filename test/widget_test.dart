import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/more/presentation/pages/chrome_extension_page.dart';
import 'package:vitapmate/features/more/presentation/pages/more_page.dart';
import 'package:vitapmate/features/more/presentation/pages/gpa_calculator_page.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

final _emptyGradeHistory = GradeHistoryData(
  student: const GradeHistoryStudentInfo(
    regNo: '',
    name: '',
    programmeBranch: '',
    programmeMode: '',
    studySystem: '',
    gender: '',
    yearJoined: '',
    eduStatus: '',
    school: '',
    campus: '',
  ),
  records: const [],
  cgpa: const GradeHistoryCgpa(
    creditsRegistered: '',
    creditsEarned: '',
    cgpa: '',
    sGrades: '',
    aGrades: '',
    bGrades: '',
    cGrades: '',
    dGrades: '',
    eGrades: '',
    fGrades: '',
    nGrades: '',
  ),
  updateTime: BigInt.zero,
);

final _emptyTimetable = TimetableData(
  slots: const [],
  courses: const [],
  semesterId: '',
  updateTime: BigInt.zero,
);

void main() {
  testWidgets('Chrome extension guide explains setup and uses Token wording', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: ChromeExtensionPage()),
        ),
      ),
    );

    expect(find.text('Copy Token'), findsOneWidget);
    expect(find.text('Reset Token'), findsOneWidget);
    expect(find.text('Open GitHub release'), findsOneWidget);
    expect(find.text('Download extension.zip'), findsOneWidget);
    expect(find.text('Open Chrome extensions'), findsOneWidget);
    expect(find.textContaining('FCM'), findsNothing);
    expect(find.textContaining('FMC'), findsNothing);
  });

  testWidgets('More page exposes the GPA and CGPA calculator', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: MorePage()),
        ),
      ),
    );

    expect(find.text('GPA / CGPA Calculator'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('GPA calculator builds its semester view', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          gradeHistoryProvider.overrideWithBuild(
            (ref, notifier) async => _emptyGradeHistory,
          ),
          timetableProvider.overrideWithBuild(
            (ref, notifier) async => _emptyTimetable,
          ),
        ],
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: GpaCalculatorPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('SEMESTER GPA'), findsOneWidget);
    expect(find.text('PROJECTED CGPA'), findsNothing);
    expect(find.text('Current semester'), findsOneWidget);
    expect(find.text('From history'), findsOneWidget);

    await tester.ensureVisible(find.text('From history'));
    await tester.tap(find.text('From history'));
    await tester.pumpAndSettle();

    expect(find.text('SEMESTER GPA'), findsNothing);
    expect(find.text('PROJECTED CGPA'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
