/// The timetable for the selected semester.
library;

import 'dart:async';

import 'package:vitapmate/features/timetable/class_reminder_scheduler.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

export 'package:vitapmate/features/timetable/class_reminder_scheduler.dart';
export 'package:vitapmate/features/timetable/logic/time_format.dart';

final timetableProvider = vtopPage<TimetableData>(
  VtopQueries.timetable,
  // Also from the saved copy: dated reminders need topping up on every
  // start. Not awaited, since it may wait on the calendar.
  onData: (ref, timetable) =>
      unawaited(rescheduleClassReminders(ref.read, timetable: timetable)),
);
