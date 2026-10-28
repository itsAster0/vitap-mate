import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/app/widgets/email_otp_banner.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/timetable/timetable_view_mode_provider.dart';
import 'package:vitapmate/features/timetable/logic/timetable_slot_merge.dart';
import 'package:vitapmate/ui/extensions.dart';
import 'package:vitapmate/features/exams/exams.dart';
import 'package:vitapmate/features/timetable/widgets/agenda_timetable_view.dart';
import 'package:vitapmate/features/timetable/widgets/weekly_timetable_view.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetablePage extends HookConsumerWidget {
  const TimetablePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedDay = useState(DateTime.now().weekday);
    final timetableData = ref.watch(timetableProvider);
    // ISO weekdays with classes, in order.
    final classDays = [
      ...{
        for (final s in timetableData.value?.slots ?? const <TimetableSlot>[])
          slotDays.indexOf(s.day) + 1,
      },
    ]..sort();
    final attendance = ref.watch(attendanceProvider).value?.records ?? const [];
    final viewMode = ref.watch(timetableViewModeProvider);
    final dragStartX = useState<double?>(null);
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        try {
          await ref.read(timetableProvider.notifier).refresh();
        } catch (e, st) {
          Log.warn('features.timetable', 'auto refresh failed: $e\n$st');
        }
        // On an exam day, pull the exam schedule too so a moved venue,
        // seat or time shows up.
        try {
          final schedule = await ref.read(cachedExamScheduleProvider.future);
          final now = DateTime.now();
          final examToday =
              [
                for (final type in schedule?.exams ?? const []) ...type.records,
              ].any(
                (exam) => switch (examDayOf(exam)) {
                  final day? =>
                    day.year == now.year &&
                        day.month == now.month &&
                        day.day == now.day,
                  null => false,
                },
              );
          if (!examToday) return;
          await ref.read(examScheduleProvider.notifier).refresh();
        } catch (e, st) {
          Log.warn(
            'features.timetable',
            'exam schedule auto refresh failed: $e\n$st',
          );
        }
      });
      return null;
    }, const []);
    final calendar = ref.watch(semesterCalendarProvider);
    // Each course's saved class history, by (course code, lab), for the
    // week recap.
    final history = {
      for (final record in attendance)
        (courseCodeOf(record), record.islab()): ?ref
            .watch(
              cachedFullAttendanceProvider(record.courseType, record.courseId),
            )
            .value,
    };
    final exams = <ExamPaper>[
      for (final type
          in ref.watch(cachedExamScheduleProvider).value?.exams ??
              const <PerExamScheduleRecord>[])
        for (final exam in type.records)
          if (examStartOf(exam) case final start?)
            (name: examName(type.examType), start: start, exam: exam),
    ]..sort((a, b) => a.start.compareTo(b.start));

    Future<void> update() async {
      try {
        await ref.read(timetableProvider.notifier).refresh();
      } catch (e) {
        Log.info('features.timetable', '$e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    final isAgenda = viewMode == .agenda;

    return ScreenRefresh(
      onRefresh: update,
      tasks: const ['vtop_timetable'],
      child: RefreshIndicator(
        displacement: 60,
        backgroundColor: context.theme.colors.background,
        color: context.theme.colors.foreground,
        strokeWidth: 2.5,
        onRefresh: update,
        child: SingleChildScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          // Swipe left/right to move between days in the agenda.
          child: GestureDetector(
            onHorizontalDragStart: isAgenda
                ? (details) => dragStartX.value = details.globalPosition.dx
                : null,
            onHorizontalDragUpdate: isAgenda
                ? (details) {
                    if (classDays.isEmpty) return;
                    final x = details.globalPosition.dx;
                    final deltaX = x - (dragStartX.value ?? x);
                    if (deltaX > 80 && classDays.first < selectedDay.value) {
                      selectedDay.value -= 1;
                      dragStartX.value = x;
                    } else if (deltaX < -80 &&
                        classDays.last > selectedDay.value) {
                      selectedDay.value += 1;
                      dragStartX.value = x;
                    }
                  }
                : null,
            onHorizontalDragEnd: isAgenda
                ? (_) => dragStartX.value = null
                : null,
            child: ConstrainedBox(
              constraints: BoxConstraints(
                minHeight: MediaQuery.sizeOf(context).height * 0.8,
              ),
              child: AnimatedSwitcher(
                duration: Motion.medium,
                // Keep content pinned to the top even when a day is short.
                layoutBuilder: (current, previous) => Stack(
                  alignment: .topCenter,
                  children: [...previous, ?current],
                ),
                child: timetableData.when(
                  skipLoadingOnRefresh: true,
                  skipLoadingOnReload: true,
                  data: (data) {
                    List<TimetableSlot> slotsForDay(int day) =>
                        mergeLabsSloths([
                          for (final slot in data.slots)
                            if (slot.day == slotDays[day - 1]) slot,
                        ]);

                    return Column(
                      key: const ValueKey('data'),
                      crossAxisAlignment: .stretch,
                      children: [
                        const EmailOtpBanner(),
                        if (viewMode == .weekly)
                          WeeklyTimetableView(
                            days: classDays,
                            slotsForDay: slotsForDay,
                            attendance: attendance,
                          )
                        else
                          AgendaTimetableView(
                            selectedDay: selectedDay,
                            classDays: classDays.toSet(),
                            slotsForDay: slotsForDay,
                            attendance: attendance,
                            calendar: calendar,
                            exams: exams,
                            history: history,
                          ),
                        DataUpdatedFooter(updateTime: data.updateTime.toInt()),
                      ],
                    );
                  },
                  error: (e, _) => EmptyState(
                    key: const ValueKey('error'),
                    icon: FLucideIcons.cloudAlert,
                    title: "Couldn't load your timetable",
                    message: commonErrorMessage(e),
                    action: FButton(
                      variant: .outline,
                      mainAxisSize: .min,
                      onPress: update,
                      child: const Text('Try again'),
                    ),
                  ),
                  loading: () =>
                      const _TimetableSkeleton(key: ValueKey('loading')),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _TimetableSkeleton extends StatelessWidget {
  const _TimetableSkeleton({super.key});

  @override
  Widget build(BuildContext context) => const Padding(
    padding: .fromLTRB(Space.sm, Space.md, Space.sm, 0),
    child: Column(
      crossAxisAlignment: .start,
      children: [
        Skeleton(width: 150, height: 30),
        SizedBox(height: Space.sm),
        Skeleton(width: 220, height: 12),
        SizedBox(height: Space.lg),
        Skeleton(height: 64, radius: Radii.md),
        SizedBox(height: Space.lg),
        Skeleton(height: 150, radius: Radii.lg),
        SizedBox(height: Space.xl),
        SkeletonList(count: 3),
      ],
    ),
  );
}
