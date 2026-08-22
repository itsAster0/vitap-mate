import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum AgendaClassStatus { completed, current, next, upcoming }

class AgendaTimetableView extends HookWidget {
  const AgendaTimetableView({
    super.key,
    required this.selectedDay,
    required this.classDays,
    required this.slots,
  });

  final ValueNotifier<int> selectedDay;
  final Set<int> classDays;
  final List<TimetableSlot> slots;

  @override
  Widget build(BuildContext context) {
    final now = useState(DateTime.now());
    useEffect(() {
      final timer = Timer.periodic(
        const Duration(minutes: 1),
        (_) => now.value = DateTime.now(),
      );
      return timer.cancel;
    }, const []);

    final sortedSlots = [...slots]
      ..sort((a, b) => _minutes(a.startTime).compareTo(_minutes(b.startTime)));
    final weekDates = _weekDates(now.value);
    final selectedDate = weekDates[selectedDay.value - 1];
    final isToday = selectedDay.value == now.value.weekday;
    final current = isToday
        ? sortedSlots.cast<TimetableSlot?>().firstWhere(
            (slot) =>
                slot != null &&
                _minutes(slot.startTime) <= _timeOfDay(now.value) &&
                _minutes(slot.endTime) > _timeOfDay(now.value),
            orElse: () => null,
          )
        : null;
    final next = sortedSlots.cast<TimetableSlot?>().firstWhere(
      (slot) =>
          slot != null &&
          (!isToday || _minutes(slot.startTime) > _timeOfDay(now.value)),
      orElse: () => null,
    );

    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 12, 8, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _DateHeading(date: selectedDate, isToday: isToday),
          const SizedBox(height: 14),
          _WeekStrip(
            dates: weekDates,
            selectedDay: selectedDay,
            classDays: classDays,
          ),
          const SizedBox(height: 14),
          _PriorityPanel(
            current: current,
            next: next,
            now: now.value,
            isToday: isToday,
          ),
          const SizedBox(height: 20),
          Row(
            children: [
              Expanded(
                child: Text(
                  isToday ? "Today's timetable" : 'Timetable',
                  style: TextStyle(
                    fontSize: 19,
                    fontWeight: FontWeight.w700,
                    color: context.theme.colors.foreground,
                  ),
                ),
              ),
              Text(
                '${sortedSlots.length} ${sortedSlots.length == 1 ? 'class' : 'classes'}',
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                  color: context.theme.colors.mutedForeground,
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          if (sortedSlots.isEmpty)
            const _EmptyDay()
          else
            for (var index = 0; index < sortedSlots.length; index++) ...[
              if (index > 0)
                _BreakRow(
                  previousEnd: sortedSlots[index - 1].endTime,
                  nextStart: sortedSlots[index].startTime,
                ),
              _AgendaClassCard(
                slot: sortedSlots[index],
                status: _statusFor(
                  sortedSlots[index],
                  current,
                  next,
                  now.value,
                  isToday,
                ),
              ),
            ],
        ],
      ),
    );
  }
}

class _DateHeading extends StatelessWidget {
  const _DateHeading({required this.date, required this.isToday});

  final DateTime date;
  final bool isToday;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                DateFormat('EEEE').format(date),
                style: TextStyle(
                  fontSize: 27,
                  height: 1.05,
                  fontWeight: FontWeight.w800,
                  color: context.theme.colors.foreground,
                ),
              ),
              const SizedBox(height: 5),
              Text(
                DateFormat('d MMMM y').format(date),
                style: TextStyle(
                  fontSize: 14,
                  color: context.theme.colors.mutedForeground,
                ),
              ),
            ],
          ),
        ),
        if (isToday)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            decoration: BoxDecoration(
              color: context.theme.colors.primary.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(999),
            ),
            child: Text(
              'TODAY',
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                color: context.theme.colors.primary,
              ),
            ),
          ),
      ],
    );
  }
}

class _WeekStrip extends StatelessWidget {
  const _WeekStrip({
    required this.dates,
    required this.selectedDay,
    required this.classDays,
  });

  final List<DateTime> dates;
  final ValueNotifier<int> selectedDay;
  final Set<int> classDays;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        for (var index = 0; index < dates.length; index++)
          Expanded(
            child: FTappable(
              semanticsLabel: DateFormat('EEEE, d MMMM').format(dates[index]),
              selected: selectedDay.value == index + 1,
              onPress: () => selectedDay.value = index + 1,
              builder: (context, variants, child) => AnimatedScale(
                scale: variants.contains(FTappableVariant.pressed) ? 0.96 : 1,
                duration: const Duration(milliseconds: 100),
                child: child,
              ),
              child: AnimatedContainer(
                duration: const Duration(milliseconds: 180),
                height: 68,
                margin: const EdgeInsets.symmetric(horizontal: 2),
                decoration: BoxDecoration(
                  color: selectedDay.value == index + 1
                      ? context.theme.colors.primary
                      : Colors.transparent,
                  borderRadius: BorderRadius.circular(13),
                ),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(
                      DateFormat('EEE').format(dates[index]).toUpperCase(),
                      style: TextStyle(
                        fontSize: 9,
                        fontWeight: FontWeight.w700,
                        color: selectedDay.value == index + 1
                            ? context.theme.colors.primaryForeground
                            : context.theme.colors.mutedForeground,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '${dates[index].day}',
                      style: TextStyle(
                        fontSize: 17,
                        fontWeight: FontWeight.w700,
                        color: selectedDay.value == index + 1
                            ? context.theme.colors.primaryForeground
                            : context.theme.colors.foreground,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Container(
                      width: 5,
                      height: 5,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: classDays.contains(index + 1)
                            ? (selectedDay.value == index + 1
                                  ? context.theme.colors.primaryForeground
                                  : context.theme.colors.primary)
                            : context.theme.colors.border,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _PriorityPanel extends StatelessWidget {
  const _PriorityPanel({
    required this.current,
    required this.next,
    required this.now,
    required this.isToday,
  });

  final TimetableSlot? current;
  final TimetableSlot? next;
  final DateTime now;
  final bool isToday;

  @override
  Widget build(BuildContext context) {
    if (current == null && next == null) {
      return Container(
        width: double.infinity,
        padding: const EdgeInsets.all(18),
        decoration: _panelDecoration(context),
        child: Row(
          children: [
            Icon(
              FLucideIcons.circleCheckBig,
              color: context.theme.colors.primary,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                isToday ? 'Classes finished for today' : 'No classes scheduled',
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  color: context.theme.colors.foreground,
                ),
              ),
            ),
          ],
        ),
      );
    }

    return Container(
      decoration: _panelDecoration(context),
      child: Column(
        children: [
          if (current != null)
            _PriorityClass(
              slot: current!,
              label: 'NOW',
              detail:
                  '${_minutes(current!.endTime) - _timeOfDay(now)} MIN LEFT',
              emphasized: true,
            ),
          if (current != null && next != null)
            Divider(height: 1, color: context.theme.colors.border),
          if (next != null)
            _PriorityClass(
              slot: next!,
              label: current == null ? 'NEXT' : 'UP NEXT',
              detail: isToday
                  ? _startsIn(next!, now)
                  : _formatRange(context, next!),
              emphasized: current == null,
            ),
        ],
      ),
    );
  }

  BoxDecoration _panelDecoration(BuildContext context) => BoxDecoration(
    color: context.theme.colors.background,
    borderRadius: BorderRadius.circular(18),
    border: Border.all(color: context.theme.colors.border),
    boxShadow: [
      BoxShadow(
        color: Colors.black.withValues(alpha: 0.045),
        blurRadius: 14,
        offset: const Offset(0, 5),
      ),
    ],
  );
}

class _PriorityClass extends StatelessWidget {
  const _PriorityClass({
    required this.slot,
    required this.label,
    required this.detail,
    required this.emphasized,
  });

  final TimetableSlot slot;
  final String label;
  final String detail;
  final bool emphasized;

  @override
  Widget build(BuildContext context) {
    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            width: 5,
            decoration: BoxDecoration(
              color: emphasized
                  ? context.theme.colors.primary
                  : const Color(0xFF6D55C5),
              borderRadius: const BorderRadius.horizontal(
                left: Radius.circular(18),
              ),
            ),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(15, 14, 14, 14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      _StatusBadge(label: label, emphasized: emphasized),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          detail,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w700,
                            color: emphasized
                                ? context.theme.colors.primary
                                : context.theme.colors.mutedForeground,
                          ),
                        ),
                      ),
                      Text(
                        _formatRange(context, slot),
                        style: TextStyle(
                          fontSize: 12,
                          fontWeight: FontWeight.w600,
                          color: context.theme.colors.foreground,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 9),
                  Text(
                    slot.name,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: emphasized ? 18 : 16,
                      height: 1.15,
                      fontWeight: FontWeight.w700,
                      color: context.theme.colors.foreground,
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text(
                    slot.courseCode,
                    style: TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                      color: context.theme.colors.mutedForeground,
                    ),
                  ),
                  const SizedBox(height: 7),
                  Row(
                    children: [
                      Icon(
                        FLucideIcons.mapPin,
                        size: 15,
                        color: context.theme.colors.primary,
                      ),
                      const SizedBox(width: 5),
                      Expanded(
                        child: Text(
                          '${slot.block} · Room ${slot.roomNo} · Slot ${slot.slot}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontSize: 13,
                            color: context.theme.colors.foreground,
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _AgendaClassCard extends StatelessWidget {
  const _AgendaClassCard({required this.slot, required this.status});

  final TimetableSlot slot;
  final AgendaClassStatus status;

  @override
  Widget build(BuildContext context) {
    final accent = switch (status) {
      AgendaClassStatus.current => context.theme.colors.primary,
      AgendaClassStatus.next => const Color(0xFF6D55C5),
      AgendaClassStatus.completed => const Color(0xFF7A8A99),
      AgendaClassStatus.upcoming => const Color(0xFF168C91),
    };
    final label = switch (status) {
      AgendaClassStatus.current => 'NOW',
      AgendaClassStatus.next => 'NEXT',
      AgendaClassStatus.completed => 'DONE',
      AgendaClassStatus.upcoming => 'UPCOMING',
    };

    return Opacity(
      opacity: status == AgendaClassStatus.completed ? 0.72 : 1,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SizedBox(
                width: 66,
                child: Padding(
                  padding: const EdgeInsets.only(top: 14, right: 10),
                  child: Text(
                    '${_formatTime(context, slot.startTime)}\n${_formatTime(context, slot.endTime)}',
                    textAlign: TextAlign.right,
                    style: TextStyle(
                      fontSize: 11,
                      height: 1.55,
                      fontWeight: status == AgendaClassStatus.current
                          ? FontWeight.w700
                          : FontWeight.w500,
                      color: status == AgendaClassStatus.current
                          ? context.theme.colors.primary
                          : context.theme.colors.mutedForeground,
                    ),
                  ),
                ),
              ),
              Container(
                width: 2,
                margin: const EdgeInsets.symmetric(vertical: 8),
                color: context.theme.colors.border,
              ),
              Expanded(
                child: Container(
                  margin: const EdgeInsets.only(left: 10),
                  padding: const EdgeInsets.fromLTRB(13, 12, 13, 12),
                  decoration: BoxDecoration(
                    color: Color.alphaBlend(
                      accent.withValues(alpha: 0.055),
                      context.theme.colors.background,
                    ),
                    borderRadius: BorderRadius.circular(14),
                    border: Border.all(color: context.theme.colors.border),
                    boxShadow: [
                      BoxShadow(color: accent, offset: const Offset(-4, 0)),
                    ],
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(
                            child: Text(
                              slot.name,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                fontSize: 15,
                                height: 1.15,
                                fontWeight: FontWeight.w700,
                                color: context.theme.colors.foreground,
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          _StatusBadge(
                            label: label,
                            emphasized: status == AgendaClassStatus.current,
                            color: accent,
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(
                        '${slot.courseCode} · Slot ${slot.slot}',
                        style: TextStyle(
                          fontSize: 12,
                          fontWeight: FontWeight.w600,
                          color: context.theme.colors.mutedForeground,
                        ),
                      ),
                      const SizedBox(height: 7),
                      Row(
                        children: [
                          Icon(FLucideIcons.mapPin, size: 14, color: accent),
                          const SizedBox(width: 5),
                          Expanded(
                            child: Text(
                              '${slot.block} · Room ${slot.roomNo}',
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                fontSize: 12,
                                color: context.theme.colors.foreground,
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Text(
                            slot.isLab ? 'LAB' : 'THEORY',
                            style: TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.w700,
                              letterSpacing: 0.5,
                              color: accent,
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({
    required this.label,
    required this.emphasized,
    this.color,
  });

  final String label;
  final bool emphasized;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final badgeColor =
        color ??
        (emphasized ? context.theme.colors.primary : const Color(0xFF6D55C5));
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 4),
      decoration: BoxDecoration(
        color: emphasized ? badgeColor : badgeColor.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(7),
        border: emphasized ? null : Border.all(color: badgeColor),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 9,
          fontWeight: FontWeight.w800,
          color: emphasized
              ? context.theme.colors.primaryForeground
              : badgeColor,
        ),
      ),
    );
  }
}

class _BreakRow extends StatelessWidget {
  const _BreakRow({required this.previousEnd, required this.nextStart});

  final String previousEnd;
  final String nextStart;

  @override
  Widget build(BuildContext context) {
    final gap = _minutes(nextStart) - _minutes(previousEnd);
    if (gap <= 0) return const SizedBox.shrink();
    final hours = gap ~/ 60;
    final minutes = gap.remainder(60);
    final duration = [
      if (hours > 0) '$hours hr',
      if (minutes > 0) '$minutes min',
    ].join(' ');

    return Padding(
      padding: const EdgeInsets.fromLTRB(78, 4, 0, 4),
      child: Container(
        height: 36,
        padding: const EdgeInsets.symmetric(horizontal: 12),
        decoration: BoxDecoration(
          color: const Color(0xFFF59E0B).withValues(alpha: 0.08),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: const Color(0xFFF59E0B).withValues(alpha: 0.22),
          ),
        ),
        child: Row(
          children: [
            const Icon(FLucideIcons.coffee, size: 15, color: Color(0xFF9A5B00)),
            const SizedBox(width: 8),
            Text(
              '$duration break',
              style: const TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w600,
                color: Color(0xFF7A4A00),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _EmptyDay extends StatelessWidget {
  const _EmptyDay();

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 34),
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Column(
        children: [
          Icon(
            FLucideIcons.calendarCheck,
            size: 30,
            color: context.theme.colors.primary,
          ),
          const SizedBox(height: 10),
          Text(
            'No classes today',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w700,
              color: context.theme.colors.foreground,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            'Your schedule is clear.',
            style: TextStyle(
              fontSize: 13,
              color: context.theme.colors.mutedForeground,
            ),
          ),
        ],
      ),
    );
  }
}

AgendaClassStatus _statusFor(
  TimetableSlot slot,
  TimetableSlot? current,
  TimetableSlot? next,
  DateTime now,
  bool isToday,
) {
  if (identical(slot, current) || slot == current) {
    return AgendaClassStatus.current;
  }
  if (identical(slot, next) || slot == next) return AgendaClassStatus.next;
  if (isToday && _minutes(slot.endTime) <= _timeOfDay(now)) {
    return AgendaClassStatus.completed;
  }
  return AgendaClassStatus.upcoming;
}

List<DateTime> _weekDates(DateTime now) {
  final monday = DateTime(
    now.year,
    now.month,
    now.day,
  ).subtract(Duration(days: now.weekday - DateTime.monday));
  return List.generate(7, (index) => monday.add(Duration(days: index)));
}

int _minutes(String time) {
  final parts = time.split(':');
  return int.parse(parts[0]) * 60 + int.parse(parts[1]);
}

int _timeOfDay(DateTime value) => value.hour * 60 + value.minute;

String _formatTime(BuildContext context, String time) {
  if (MediaQuery.alwaysUse24HourFormatOf(context)) return time;
  final parts = time.split(':');
  final hour24 = int.parse(parts[0]);
  final hour12 = hour24 % 12 == 0 ? 12 : hour24 % 12;
  return '$hour12:${parts[1]} ${hour24 >= 12 ? 'PM' : 'AM'}';
}

String _formatRange(BuildContext context, TimetableSlot slot) =>
    '${_formatTime(context, slot.startTime)} – ${_formatTime(context, slot.endTime)}';

String _startsIn(TimetableSlot slot, DateTime now) {
  final difference = _minutes(slot.startTime) - _timeOfDay(now);
  if (difference <= 0) return 'STARTING NOW';
  if (difference < 60) return 'IN $difference MIN';
  final hours = difference ~/ 60;
  final minutes = difference.remainder(60);
  return minutes == 0 ? 'IN $hours HR' : 'IN $hours HR $minutes MIN';
}
