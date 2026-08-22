import 'dart:developer';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_view_mode_provider.dart';
import 'package:vitapmate/features/timetable/presentation/utils/timetable_slot_merge.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/days_stack.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/agenda_timetable_view.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/timetable_card.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/timetable_colors.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/weekly_timetable_view.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class TimetablePage extends HookConsumerWidget {
  const TimetablePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final key = useMemoized(() => GlobalKey());
    final selectedDay = useState<int>(DateTime.now().weekday);
    final finalDay = useState<List<int>>([]);
    final scrollController = useScrollController();
    final scrollOffset = useState<double>(0);
    final timetableData = ref.watch(timetableProvider);
    final viewMode = ref.watch(timetableViewModeProvider);
    final autoRefresh = ref.watch(autoRefreshProvider);
    final startX = useState<double?>(null);
    useEffect(() {
      if (!autoRefresh) return null;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        ref.read(timetableProvider.notifier).updateTimetable().catchError((
          e,
          st,
        ) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });

      return null;
    }, [autoRefresh]);
    final mergeLabs = ref.watch(mergeTTProvider);

    useEffect(() {
      if (!timetableData.hasValue) return null;

      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!scrollController.hasClients) return;
        final activeMode = ref.read(timetableViewModeProvider);
        final targetOffset = activeMode == TimetableViewMode.daily ? 72.0 : 0.0;
        scrollController.jumpTo(
          targetOffset
              .clamp(0.0, scrollController.position.maxScrollExtent)
              .toDouble(),
        );
      });
      return null;
    }, [viewMode, timetableData.hasValue]);

    Future<void> update() async {
      try {
        await ref.read(timetableProvider.notifier).updateTimetable();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    return Container(
      decoration: BoxDecoration(),
      child: Stack(
        children: [
          RefreshIndicator(
            displacement: 120,
            key: key,
            backgroundColor: context.theme.colors.primary,
            color: context.theme.colors.primaryForeground,
            strokeWidth: 2.5,
            onRefresh: update,
            child: SingleChildScrollView(
              controller: scrollController,
              physics: const AlwaysScrollableScrollPhysics(),
              child: GestureDetector(
                onHorizontalDragStart: viewMode != TimetableViewMode.weekly
                    ? (details) {
                        startX.value = details.globalPosition.dx;
                      }
                    : null,
                onHorizontalDragUpdate: viewMode != TimetableViewMode.weekly
                    ? (details) {
                        if (finalDay.value.isEmpty) return;
                        final currentX = details.globalPosition.dx;
                        final deltaX = currentX - (startX.value ?? currentX);

                        if (deltaX > 80 &&
                            finalDay.value.first < selectedDay.value) {
                          selectedDay.value -= 1;
                          startX.value = currentX;
                        } else if (deltaX < -80 &&
                            finalDay.value.last > selectedDay.value) {
                          selectedDay.value += 1;
                          startX.value = currentX;
                        }
                      }
                    : null,
                onHorizontalDragEnd: viewMode != TimetableViewMode.weekly
                    ? (_) => startX.value = null
                    : null,
                child: ConstrainedBox(
                  constraints: BoxConstraints(
                    minHeight: MediaQuery.of(context).size.height * 0.8,
                  ),
                  child: timetableData.when(
                    data: (data) {
                      final tempList = getDayList(data);
                      finalDay.value = tempList;

                      if (viewMode == TimetableViewMode.daily &&
                          !tempList.contains(selectedDay.value)) {
                        selectedDay.value = tempList.isEmpty
                            ? DateTime.now().weekday
                            : tempList.first;
                      }
                      List<TimetableSlot> slotsForDay(int day) {
                        var slots = getDaySlotList(data, day);
                        if (mergeLabs) {
                          slots = mergeLabsSloths(slots);
                        }
                        slots.sort((a, b) {
                          final t1 = _parseTime(a.startTime);
                          final t2 = _parseTime(b.startTime);
                          return t1.compareTo(t2);
                        });
                        return slots;
                      }

                      if (viewMode == TimetableViewMode.weekly) {
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            WeeklyTimetableView(
                              days: tempList,
                              slotsForDay: slotsForDay,
                            ),
                            DataUpdatedFooter(
                              updateTime: data.updateTime.toInt(),
                            ),
                          ],
                        );
                      }

                      if (viewMode == TimetableViewMode.agenda) {
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            AgendaTimetableView(
                              selectedDay: selectedDay,
                              classDays: tempList.toSet(),
                              slots: slotsForDay(selectedDay.value),
                            ),
                            DataUpdatedFooter(
                              updateTime: data.updateTime.toInt(),
                            ),
                          ],
                        );
                      }

                      final tempdays = slotsForDay(selectedDay.value);

                      final daySlots = addFreeSlots(tempdays);

                      daySlots.sort((a, b) {
                        final t1 = _parseTime(a.startTime);
                        final t2 = _parseTime(b.startTime);
                        return t1.compareTo(t2);
                      });
                      return Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          const SizedBox(height: 72),
                          TimetableDaySummary(slots: tempdays),
                          ...daySlots.map((slot) => TimetableCard(slot: slot)),
                          DataUpdatedFooter(
                            updateTime: data.updateTime.toInt(),
                          ),
                        ],
                      );
                    },
                    error: (e, stackTrace) {
                      return Center(
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Icon(
                              Icons.error_outline,
                              size: 48,
                              color: Colors.grey[400],
                            ),
                            const SizedBox(height: 16),
                            Text(
                              commonErrorMessage(e),
                              textAlign: TextAlign.center,
                              style: TextStyle(
                                fontSize: 16,
                                color: Colors.grey[600],
                              ),
                            ),
                          ],
                        ),
                      );
                    },
                    loading: () => const Center(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          SizedBox(
                            width: 40,
                            height: 40,
                            child: CircularProgressIndicator(
                              strokeWidth: 3,
                              valueColor: AlwaysStoppedAnimation<Color>(
                                TimetableColors.upcomingBorder,
                              ),
                            ),
                          ),
                          SizedBox(height: 16),
                          Text(
                            'Loading timetable...',
                            style: TextStyle(
                              fontSize: 14,
                              color: Colors.grey,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
          if (timetableData.hasValue && viewMode == TimetableViewMode.daily)
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              child: FrostedGlassBox(
                child: DaysStack(
                  selectedDay: selectedDay,
                  daysList: getDayList(timetableData.value),
                ),
              ),
            ),
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

List<TimetableSlot> addFreeSlots(List<TimetableSlot> t) {
  if (t.isEmpty) return t;

  List<TimetableSlot> r = [];
  for (int i = 0; i < t.length - 1; i++) {
    r.add(t[i]);
    final cClass = t[i].endTime;
    final nClass = t[i + 1].startTime;
    int diff = getdiff(cClass, nClass);
    int mod = (diff / 30).toInt();

    if (mod > 0) {
      r.add(
        TimetableSlot(
          serial: "-1",
          day: "-",
          slot: _formatHours(mod * 30),
          courseCode: "-",
          courseType: "-",
          roomNo: "-",
          block: "-",
          startTime: cClass,
          endTime: nClass,
          name: "-",
          isLab: false,
          faculty: '',
          credits: '',
        ),
      );
    }
  }
  r.add(t[t.length - 1]);
  return r;
}

String _formatHours(int minutes) {
  final hours = minutes / 60;
  return hours == hours.roundToDouble()
      ? hours.toInt().toString()
      : hours.toStringAsFixed(1);
}

class TimetableDaySummary extends StatelessWidget {
  const TimetableDaySummary({super.key, required this.slots});

  final List<TimetableSlot> slots;

  @override
  Widget build(BuildContext context) {
    final classSlots = slots.where((slot) => slot.serial != "-1").toList();
    final scheduledHours = classSlots.fold<int>(
      0,
      (total, slot) => total + _scheduledHours(slot),
    );
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: context.theme.colors.background,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: context.theme.colors.border),
        ),
        child: Row(
          children: [
            Expanded(
              child: _summaryMetric(
                context,
                FLucideIcons.clock,
                scheduledHours.toString(),
                "scheduled hours",
              ),
            ),
            Container(width: 1, height: 42, color: context.theme.colors.border),
            Expanded(
              child: _summaryMetric(
                context,
                FLucideIcons.bookOpen,
                classSlots.length.toString(),
                classSlots.length == 1 ? "class" : "classes",
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _summaryMetric(
    BuildContext context,
    IconData icon,
    String value,
    String label,
  ) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Container(
          width: 32,
          height: 32,
          decoration: BoxDecoration(
            color: context.theme.colors.primary.withValues(alpha: 0.12),
            shape: BoxShape.circle,
          ),
          child: Icon(icon, size: 16, color: context.theme.colors.primary),
        ),
        const SizedBox(width: 8),
        Flexible(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                value,
                style: TextStyle(
                  fontSize: 17,
                  height: 1,
                  fontWeight: FontWeight.w700,
                  color: context.theme.colors.foreground,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 11,
                  color: context.theme.colors.foreground.withValues(
                    alpha: 0.65,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

int _scheduledHours(TimetableSlot slot) {
  if (!slot.isLab) return 1;
  return slot.slot.split('+').length;
}

int getdiff(String a, String b) {
  var first = a.split(":");
  var second = b.split(":");
  return (int.parse(second[0]) * 60 + int.parse(second[1])) -
      (int.parse(first[0]) * 60 + int.parse(first[1]));
}

Duration _parseTime(String t) {
  final parts = t.split(":");
  final h = int.parse(parts[0]);
  final m = parts.length > 1 ? int.parse(parts[1]) : 0;
  return Duration(hours: h, minutes: m);
}

class FrostedGlassBox extends StatelessWidget {
  final Widget child;

  const FrostedGlassBox({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: AlignmentGeometry.topCenter,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(20),
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: 10, sigmaY: 10),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10.0, vertical: 6),
            child: child,
          ),
        ),
      ),
    );
  }
}
