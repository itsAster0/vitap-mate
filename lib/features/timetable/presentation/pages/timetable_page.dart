import 'dart:developer';

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/email_otp_banner.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/calendar/presentation/providers/academic_calendar_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_view_mode_provider.dart';
import 'package:vitapmate/features/timetable/presentation/utils/timetable_slot_merge.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/agenda_timetable_view.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/weekly_timetable_view.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetablePage extends HookConsumerWidget {
  const TimetablePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedDay = useState<int>(DateTime.now().weekday);
    final timetableData = ref.watch(timetableProvider);
    final classDays = getDayList(timetableData.value);
    final attendance = ref.watch(attendanceProvider).value?.records ?? const [];
    final viewMode = ref.watch(timetableViewModeProvider);
    final dragStartX = useState<double?>(null);
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(timetableProvider.notifier).updateTimetable().catchError((
          e,
          st,
        ) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });

      return null;
    }, const []);
    final calendar = ref.watch(semesterCalendarProvider);

    Future<void> update() async {
      try {
        await ref.read(timetableProvider.notifier).updateTimetable();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    final isAgenda = viewMode == TimetableViewMode.agenda;

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
                    final days = classDays;
                    if (days.isEmpty) return;
                    final currentX = details.globalPosition.dx;
                    final deltaX = currentX - (dragStartX.value ?? currentX);
                    if (deltaX > 80 && days.first < selectedDay.value) {
                      selectedDay.value -= 1;
                      dragStartX.value = currentX;
                    } else if (deltaX < -80 && days.last > selectedDay.value) {
                      selectedDay.value += 1;
                      dragStartX.value = currentX;
                    }
                  }
                : null,
            onHorizontalDragEnd: isAgenda
                ? (_) => dragStartX.value = null
                : null,
            child: ConstrainedBox(
              constraints: BoxConstraints(
                minHeight: MediaQuery.of(context).size.height * 0.8,
              ),
              child: AnimatedSwitcher(
                duration: Motion.medium,
                // Keep content pinned to the top even when a day is short.
                layoutBuilder: (current, previous) => Stack(
                  alignment: Alignment.topCenter,
                  children: [...previous, ?current],
                ),
                child: timetableData.when(
                  skipLoadingOnRefresh: true,
                  skipLoadingOnReload: true,
                  data: (data) {
                    final days = classDays;

                    List<TimetableSlot> slotsForDay(int day) {
                      final slots = mergeLabsSloths(getDaySlotList(data, day));
                      slots.sort(
                        (a, b) => _parseTime(
                          a.startTime,
                        ).compareTo(_parseTime(b.startTime)),
                      );
                      return slots;
                    }

                    return Column(
                      key: const ValueKey('data'),
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const EmailOtpBanner(),
                        if (viewMode == TimetableViewMode.weekly)
                          WeeklyTimetableView(
                            days: days,
                            slotsForDay: slotsForDay,
                            attendance: attendance,
                          )
                        else
                          AgendaTimetableView(
                            selectedDay: selectedDay,
                            classDays: days.toSet(),
                            slotsForDay: slotsForDay,
                            attendance: attendance,
                            calendar: calendar,
                          ),
                        DataUpdatedFooter(updateTime: data.updateTime.toInt()),
                      ],
                    );
                  },
                  error: (e, stackTrace) => EmptyState(
                    key: const ValueKey('error'),
                    icon: FLucideIcons.cloudAlert,
                    title: "Couldn't load your timetable",
                    message: commonErrorMessage(e),
                    action: FButton(
                      variant: FButtonVariant.outline,
                      mainAxisSize: MainAxisSize.min,
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
  Widget build(BuildContext context) {
    return const Padding(
      padding: EdgeInsets.fromLTRB(Space.sm, Space.md, Space.sm, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
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
}

List<int> getDayList(TimetableData? data) {
  if (data == null) return [];
  Map<String, int> map = {
    "MON": 1,
    "TUE": 2,
    "WED": 3,
    "THU": 4,
    "FRI": 5,
    "SAT": 6,
    "SUN": 7,
  };
  Set found = {};
  for (final i in data.slots) {
    if (!found.contains(i.day)) {
      found.add(i.day);
    }
  }
  var out = found.map((k) => map[k]!).toList();
  out.sort();
  return out;
}

List<TimetableSlot> getDaySlotList(TimetableData data, int i) {
  Map<int, String> map = {
    1: "MON",
    2: "TUE",
    3: "WED",
    4: "THU",
    5: "FRI",
    6: "SAT",
    7: "SUN",
  };
  String day = map[i]!;
  List<TimetableSlot> slots = [];
  for (final slot in data.slots) {
    if (slot.day == day) {
      slots.add(slot);
    }
  }
  return slots;
}

Duration _parseTime(String t) {
  final parts = t.split(":");
  final h = int.parse(parts[0]);
  final m = parts.length > 1 ? int.parse(parts[1]) : 0;
  return Duration(hours: h, minutes: m);
}
