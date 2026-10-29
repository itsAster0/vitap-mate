/// The exam schedule for the selected semester.
library;

import 'dart:async';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/platform/notifications/exam_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

export 'package:vitapmate/features/exams/logic/exam_time.dart';

final examScheduleProvider = vtopPage<ExamScheduleData>(
  VtopQueries.examSchedule,
  onData: (ref, schedule) {
    unawaited(ExamReminderNotificationService.syncFromExamSchedule(schedule));
    ref.invalidate(cachedExamScheduleProvider);
  },
);

/// The exam schedule as last saved, or null if it was never fetched. Never
/// goes to VTOP, for screens that only show it in passing.
final cachedExamScheduleProvider = FutureProvider(
  (ref) => readSavedSemesterPage(ref.watch, VtopQueries.examSchedule),
);
