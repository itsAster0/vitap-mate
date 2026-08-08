import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/theme.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

const Color _urgentColor = Color(0xFFD32F2F);
const Color _laterColor = Color(0xFF2E7D32);

DateTime? examDateTimeOf(ExamScheduleRecord exam) {
  DateTime base;
  try {
    base = DateTime.parse(exam.examDate);
  } catch (_) {
    return null;
  }
  final time = _parseTimeOfDay(exam.examTime);
  if (time == null) {
    return DateTime(base.year, base.month, base.day, 23, 59);
  }
  return DateTime(base.year, base.month, base.day, time.$1, time.$2);
}

(int, int)? _parseTimeOfDay(String raw) {
  final match = RegExp(
    r'^\s*(\d{1,2})(?::(\d{2}))?\s*([AaPp][Mm])?\s*$',
  ).firstMatch(raw);
  if (match == null) return null;
  var hour = int.tryParse(match.group(1) ?? '');
  final minute = int.tryParse(match.group(2) ?? '') ?? 0;
  final period = match.group(3)?.toUpperCase();
  if (hour == null || hour > 23 || minute > 59) return null;
  if (period == 'AM') {
    if (hour == 12) hour = 0;
  } else if (period == 'PM') {
    if (hour != 12) hour += 12;
  }
  return (hour, minute);
}

String relativeDayLabel(DateTime dt, {DateTime? now}) {
  final ref = now ?? DateTime.now();
  final today = DateTime(ref.year, ref.month, ref.day);
  final target = DateTime(dt.year, dt.month, dt.day);
  if (!target.isAfter(today)) return "TODAY";
  if (target.isAtSameMomentAs(today.add(const Duration(days: 1)))) {
    return "TOMORROW";
  }
  final days = dt.difference(ref).inDays;
  return "in ${days < 1 ? 1 : days}d";
}

Color urgencyColorFor(DateTime dt) {
  final remaining = dt.difference(DateTime.now());
  if (remaining <= const Duration(hours: 48)) return _urgentColor;
  if (remaining <= const Duration(days: 7)) return ExamColors.todayText;
  return _laterColor;
}

({String label, Color color})? nearestUpcoming(
  List<PerExamScheduleRecord> exams,
) {
  DateTime? nearest;
  for (final record in exams) {
    for (final exam in record.records) {
      final dt = examDateTimeOf(exam);
      if (dt == null || !dt.isAfter(DateTime.now())) continue;
      if (nearest == null || dt.isBefore(nearest)) nearest = dt;
    }
  }
  if (nearest == null) return null;
  return (label: relativeDayLabel(nearest), color: urgencyColorFor(nearest));
}

List<(ExamScheduleRecord, DateTime)> _upcomingExams(
  List<PerExamScheduleRecord> exams,
  int limit,
) {
  final now = DateTime.now();
  final result = <(ExamScheduleRecord, DateTime)>[];
  for (final record in exams) {
    for (final exam in record.records) {
      final dt = examDateTimeOf(exam);
      if (dt == null || !dt.isAfter(now)) continue;
      result.add((exam, dt));
    }
  }
  result.sort((a, b) => a.$2.compareTo(b.$2));
  return result.take(limit).toList();
}

class ExamCountdownStrip extends HookConsumerWidget {
  final List<PerExamScheduleRecord> exams;

  const ExamCountdownStrip({super.key, required this.exams});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final tick = useState(0);

    useEffect(() {
      final timer = Timer.periodic(const Duration(minutes: 1), (_) {
        tick.value++;
      });
      return timer.cancel;
    }, const []);

    final controller = useAnimationController(
      duration: const Duration(milliseconds: 400),
    );
    useEffect(() {
      controller.forward();
      return null;
    }, const []);

    final upcoming = useMemoized(
      () => _upcomingExams(exams, 8),
      [exams, tick.value],
    );

    if (upcoming.isEmpty) return const SizedBox.shrink();

    return RepaintBoundary(
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        clipBehavior: Clip.none,
        child: Row(
          spacing: 8,
          children: [
            for (var i = 0; i < upcoming.length; i++)
              RepaintBoundary(
                child: _CountdownChip(
                  parent: controller,
                  curve: Interval(
                    i * 0.06,
                    (i * 0.06 + 0.5).clamp(0.0, 1.0).toDouble(),
                    curve: Curves.easeOutCubic,
                  ),
                  exam: upcoming[i].$1,
                  dateTime: upcoming[i].$2,
                  darkMode: darkMode,
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _CountdownChip extends StatelessWidget {
  final Animation<double> parent;
  final Interval curve;
  final ExamScheduleRecord exam;
  final DateTime dateTime;
  final bool darkMode;

  const _CountdownChip({
    required this.parent,
    required this.curve,
    required this.exam,
    required this.dateTime,
    required this.darkMode,
  });

  @override
  Widget build(BuildContext context) {
    final color = urgencyColorFor(dateTime);
    return AnimatedBuilder(
      animation: parent,
      builder: (context, child) {
        final t = curve.transform(parent.value);
        return Opacity(
          opacity: t,
          child: Transform.translate(
            offset: Offset(0, 12 * (1 - t)),
            child: child,
          ),
        );
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          color: color.withValues(alpha: darkMode ? 0.18 : 0.12),
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: color.withValues(alpha: 0.35)),
          boxShadow: [
            BoxShadow(
              color: ExamColors.cardShadow,
              blurRadius: 6,
              offset: const Offset(0, 2),
            ),
          ],
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              exam.courseCode,
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w700,
                color: darkMode
                    ? context.theme.colors.primary
                    : ExamColors.primaryText,
              ),
            ),
            Text(
              exam.courseType,
              style: TextStyle(
                fontSize: 10,
                fontWeight: FontWeight.w500,
                color: darkMode
                    ? context.theme.colors.mutedForeground
                    : ExamColors.secondaryText,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              relativeDayLabel(dateTime),
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.3,
                color: color,
              ),
            ),
            Text(
              DateFormat('dd MMM').format(dateTime),
              style: TextStyle(
                fontSize: 10,
                fontWeight: FontWeight.w500,
                color: darkMode
                    ? context.theme.colors.mutedForeground
                    : ExamColors.tertiaryText,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
