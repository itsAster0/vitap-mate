import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/features/more/domain/gpa_calculator.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/gpa_dial.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';

class _Row {
  final int id;
  final double credits;
  final String grade;
  final String courseCode;
  final String courseName;

  const _Row({
    required this.id,
    this.credits = 4,
    this.grade = 'A',
    this.courseCode = '',
    this.courseName = '',
  });

  _Row copyWith({double? credits, String? grade}) => _Row(
    id: id,
    credits: credits ?? this.credits,
    grade: grade ?? this.grade,
    courseCode: courseCode,
    courseName: courseName,
  );
}

String _formatCredits(double credits) => credits == credits.roundToDouble()
    ? credits.toStringAsFixed(0)
    : credits.toStringAsFixed(1);

class GpaCalculatorPage extends HookConsumerWidget {
  const GpaCalculatorPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hasHistory = useState(false);
    final rows = useState<List<_Row>>([const _Row(id: 1)]);
    final nextId = useState(2);
    final cgpaController = useTextEditingController();
    final earnedController = useTextEditingController();
    useListenable(cgpaController);
    useListenable(earnedController);
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final history = ref.watch(gradeHistoryProvider);
    final timetable = ref.watch(timetableProvider);

    void addRow() {
      rows.value = [...rows.value, _Row(id: nextId.value)];
      nextId.value++;
    }

    void updateRow(int id, {required _Row Function(_Row) transform}) {
      rows.value = [
        for (final r in rows.value)
          if (r.id == id) transform(r) else r,
      ];
    }

    void removeRow(int id) {
      if (rows.value.length == 1) return;
      rows.value = rows.value.where((r) => r.id != id).toList();
    }

    void addFromHistory() {
      final data = history.value;
      if (data == null) return;

      cgpaController.text = data.cgpa.cgpa;
      earnedController.text = data.cgpa.creditsRegistered;
      hasHistory.value = true;
    }

    void addCurrentSemester() {
      final data = timetable.value;
      if (data == null) return;
      final courses = combineSemesterCourseCredits([
        for (final course in data.courses)
          if (parseCredits(course.credits) case final credits?)
            SemesterCourseComponent(
              courseCode: course.courseCode,
              courseType: course.courseType,
              credits: credits,
            ),
      ]);
      if (courses.isEmpty) return;
      final courseNames = {
        for (final course in data.courses)
          course.courseCode.trim().toUpperCase(): course.name.trim(),
      };

      var uid = nextId.value + 1000;
      rows.value = [
        for (final course in courses)
          _Row(
            id: uid++,
            credits: course.credits,
            courseCode: course.courseCode,
            courseName: courseNames[course.courseCode] ?? '',
          ),
      ];
      nextId.value = uid;
    }

    final courses = [
      for (final row in rows.value)
        GpaCourse(credits: row.credits, grade: row.grade),
    ];
    final sumCredits = courses.fold<double>(
      0,
      (total, course) => total + course.credits,
    );
    final semesterGpa = calculateSemesterGpa(courses);

    final currentCgpa =
        (double.tryParse(cgpaController.text.replaceAll(',', '.')) ?? 0.0)
            .clamp(0.0, 10.0);
    final earned = parseCredits(earnedController.text) ?? 0.0;
    final projected = calculateProjectedCgpa(
      currentCgpa: currentCgpa,
      completedCredits: earned,
      plannedCourses: courses,
    );
    final delta = projected - currentCgpa;

    return SingleChildScrollView(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(0, 8, 0, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Center(
            child: AnimatedSwitcher(
              duration: const Duration(milliseconds: 300),
              child: hasHistory.value
                  ? GpaDial(
                      key: const ValueKey('dial-cgpa'),
                      value: projected,
                      label: 'PROJECTED CGPA',
                      caption: delta.abs() < 0.005
                          ? 'no change'
                          : '${delta > 0 ? '+' : ''}${delta.toStringAsFixed(2)}',
                    )
                  : GpaDial(
                      key: const ValueKey('dial-gpa'),
                      value: semesterGpa,
                      label: 'SEMESTER GPA',
                      caption: '${_formatCredits(sumCredits)} credits',
                    ),
            ),
          ),
          if (hasHistory.value) ...[
            const SizedBox(height: 14),
            Center(
              child: TweenAnimationBuilder<double>(
                tween: Tween(begin: 0, end: delta),
                duration: const Duration(milliseconds: 500),
                curve: Curves.easeOutCubic,
                builder: (context, d, _) {
                  if (d.abs() < 0.005 || currentCgpa <= 0) {
                    return const SizedBox.shrink();
                  }
                  final up = d > 0;
                  final c = up
                      ? const Color(0xFF2E7D32)
                      : const Color(0xFFD32F2F);
                  return Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 6,
                    ),
                    decoration: BoxDecoration(
                      color: c.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(999),
                      border: Border.all(color: c.withValues(alpha: 0.35)),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          up
                              ? FLucideIcons.trendingUp
                              : FLucideIcons.trendingDown,
                          size: 14,
                          color: c,
                        ),
                        const SizedBox(width: 6),
                        Text(
                          '${up ? '+' : ''}${d.toStringAsFixed(2)} from current ${currentCgpa.toStringAsFixed(2)}',
                          style: TextStyle(
                            fontSize: 12.5,
                            fontWeight: FontWeight.w700,
                            color: c,
                            fontFeatures: const [FontFeature.tabularFigures()],
                          ),
                        ),
                      ],
                    ),
                  );
                },
              ),
            ),
          ],
          const SizedBox(height: 18),
          if (hasHistory.value)
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: darkMode
                    ? context.theme.colors.primaryForeground
                    : const Color(0xFFF5F5F5),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: context.theme.colors.border),
              ),
              child: Row(
                children: [
                  Expanded(
                    child: FTextField(
                      label: const Text('Current CGPA'),
                      hint: 'e.g. 8.35',
                      keyboardType: const TextInputType.numberWithOptions(
                        decimal: true,
                      ),
                      control: FTextFieldControl.managed(
                        controller: cgpaController,
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: FTextField(
                      label: const Text('Completed credits'),
                      hint: 'e.g. 54',
                      keyboardType: TextInputType.number,
                      control: FTextFieldControl.managed(
                        controller: earnedController,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          const SizedBox(height: 14),
          AnimatedSize(
            duration: const Duration(milliseconds: 250),
            curve: Curves.easeOutCubic,
            alignment: Alignment.topCenter,
            child: Column(
              children: [
                for (var i = 0; i < rows.value.length; i++)
                  _CourseRowTile(
                    key: ValueKey(rows.value[i].id),
                    index: i,
                    row: rows.value[i],
                    canRemove: rows.value.length > 1,
                    onCredits: (c) => updateRow(
                      rows.value[i].id,
                      transform: (r) => r.copyWith(credits: c),
                    ),
                    onGrade: (g) => updateRow(
                      rows.value[i].id,
                      transform: (r) => r.copyWith(grade: g),
                    ),
                    onRemove: () => removeRow(rows.value[i].id),
                  ),
              ],
            ),
          ),
          const SizedBox(height: 10),
          Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _actionButton(
                context,
                label: 'Add course',
                icon: FLucideIcons.plus,
                onPress: addRow,
              ),
              const SizedBox(height: 10),
              Row(
                children: [
                  Expanded(
                    child: _actionButton(
                      context,
                      label: 'Current semester',
                      icon: FLucideIcons.calendarDays,
                      onPress: addCurrentSemester,
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: _actionButton(
                      context,
                      label: 'From history',
                      icon: FLucideIcons.history,
                      onPress: addFromHistory,
                      muted: true,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _actionButton(
    BuildContext context, {
    required String label,
    required IconData icon,
    required VoidCallback onPress,
    bool muted = false,
  }) {
    final color = muted
        ? context.theme.colors.mutedForeground
        : context.theme.colors.primary;
    return FTappable(
      onPress: onPress,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(10),
          border: Border.all(color: context.theme.colors.border, width: 1.5),
        ),
        child: FittedBox(
          fit: BoxFit.scaleDown,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 16, color: color),
              const SizedBox(width: 6),
              Text(
                label,
                style: TextStyle(fontWeight: FontWeight.w600, color: color),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _CourseRowTile extends StatelessWidget {
  final int index;
  final _Row row;
  final bool canRemove;
  final ValueChanged<double> onCredits;
  final ValueChanged<String> onGrade;
  final VoidCallback onRemove;

  const _CourseRowTile({
    super.key,
    required this.index,
    required this.row,
    required this.canRemove,
    required this.onCredits,
    required this.onGrade,
    required this.onRemove,
  });

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Container(
        padding: const EdgeInsets.fromLTRB(10, 8, 6, 8),
        decoration: BoxDecoration(
          color: context.theme.colors.primaryForeground,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: context.theme.colors.border),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (row.courseCode.isNotEmpty) ...[
              Text(
                row.courseName.isEmpty
                    ? row.courseCode
                    : '${row.courseCode} - ${row.courseName}',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w700,
                  color: context.theme.colors.mutedForeground,
                ),
              ),
              const SizedBox(height: 6),
            ],
            Row(
              children: [
                Text(
                  '${index + 1}',
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w700,
                    color: context.theme.colors.mutedForeground,
                  ),
                ),
                const SizedBox(width: 8),
                FTappable(
                  onPress: () => onCredits((row.credits - 0.5).clamp(0.5, 30)),
                  child: Icon(
                    FLucideIcons.minus,
                    size: 15,
                    color: context.theme.colors.mutedForeground,
                  ),
                ),
                SizedBox(
                  width: 52,
                  child: Column(
                    children: [
                      Text(
                        _formatCredits(row.credits),
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          fontSize: 15,
                          fontWeight: FontWeight.w700,
                          color: context.theme.colors.primary,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                      Text(
                        'credits',
                        style: TextStyle(
                          fontSize: 9.5,
                          color: context.theme.colors.mutedForeground,
                        ),
                      ),
                    ],
                  ),
                ),
                FTappable(
                  onPress: () => onCredits((row.credits + 0.5).clamp(0.5, 30)),
                  child: Icon(
                    FLucideIcons.plus,
                    size: 15,
                    color: context.theme.colors.primary,
                  ),
                ),
                const Spacer(),
                SizedBox(
                  width: 108,
                  height: 40,
                  child: FSelect<String>(
                    size: .sm,
                    items: {
                      for (final g in gradePoints.keys)
                        '$g (${gradePoints[g]})': g,
                    },
                    control: FSelectControl.lifted(
                      value: row.grade,
                      onChange: (g) {
                        if (g != null) onGrade(g);
                      },
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                if (canRemove)
                  FTappable(
                    onPress: onRemove,
                    child: Padding(
                      padding: const EdgeInsets.all(6),
                      child: Icon(
                        FLucideIcons.trash2,
                        size: 15,
                        color: context.theme.colors.mutedForeground,
                      ),
                    ),
                  )
                else
                  const SizedBox(width: 27),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
