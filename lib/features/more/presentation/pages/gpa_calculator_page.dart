import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/more/domain/gpa_calculator.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/grade_badge.dart';
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

/// Grades offered in the picker (N carries no points and is left out).
const _pickerGrades = ['S', 'A', 'B', 'C', 'D', 'E', 'F'];

class GpaCalculatorPage extends HookConsumerWidget {
  const GpaCalculatorPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final rows = useState<List<_Row>>([const _Row(id: 1)]);
    final nextId = useState(2);
    final cgpaController = useTextEditingController();
    final earnedController = useTextEditingController();
    final editBase = useState(false);
    useListenable(cgpaController);
    useListenable(earnedController);
    final history = ref.watch(gradeHistoryProvider);
    final timetable = ref.watch(timetableProvider);

    // Prefill current CGPA and credits from grade history once it's loaded.
    final prefilled = useState(false);
    useEffect(() {
      final data = history.value;
      if (data != null && !prefilled.value) {
        prefilled.value = true;
        if (cgpaController.text.isEmpty) cgpaController.text = data.cgpa.cgpa;
        if (earnedController.text.isEmpty) {
          earnedController.text = data.cgpa.creditsRegistered;
        }
      }
      return null;
    }, [history.value]);

    void addRow() {
      rows.value = [...rows.value, _Row(id: nextId.value)];
      nextId.value++;
    }

    void updateRow(int id, _Row Function(_Row) transform) {
      rows.value = [
        for (final r in rows.value)
          if (r.id == id) transform(r) else r,
      ];
    }

    void removeRow(int id) {
      if (rows.value.length == 1) return;
      rows.value = rows.value.where((r) => r.id != id).toList();
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
            credits: course.credits.value,
            courseCode: course.courseCode,
            courseName: courseNames[course.courseCode] ?? '',
          ),
      ];
      nextId.value = uid;
    }

    final courses = [
      for (final row in rows.value)
        GpaCourse(
          credits: Credits(row.credits),
          grade: Grade.tryParse(row.grade)!,
        ),
    ];
    final sumCredits = courses.fold<double>(0, (t, c) => t + c.credits.value);
    final semesterGpa = calculateSemesterGpa(courses);
    final currentCgpa = Cgpa.tryParse(cgpaController.text);
    final earned = parseCredits(earnedController.text)?.value ?? 0.0;
    final projected = currentCgpa == null
        ? null
        : calculateProjectedCgpa(
            currentCgpa: currentCgpa,
            completedCredits: earned,
            plannedCourses: courses,
          );
    final delta = projected == null ? 0.0 : projected - currentCgpa!.value;
    final gpaTone = gradeTone(context, _letterFor(semesterGpa));

    return ListView(
      padding: const EdgeInsets.fromLTRB(
        Space.sm,
        Space.sm,
        Space.sm,
        Space.xl,
      ),
      children: [
        // Summary: semester GPA ring + CGPA now → after.
        Surface(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  ProgressRing(
                    value: semesterGpa / 10,
                    color: gpaTone.base,
                    size: 84,
                    stroke: 7,
                    child: Text(
                      semesterGpa.toStringAsFixed(2),
                      style: typography.body.lg.copyWith(
                        fontWeight: FontWeight.w600,
                        color: colors.foreground,
                        fontFeatures: const [FontFeature.tabularFigures()],
                      ),
                    ),
                  ),
                  const SizedBox(width: Space.lg),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'SEMESTER GPA',
                          style: typography.body.xs.copyWith(
                            fontWeight: FontWeight.w600,
                            letterSpacing: 0.6,
                            color: colors.mutedForeground,
                          ),
                        ),
                        Text(
                          '${rows.value.length} ${rows.value.length == 1 ? 'course' : 'courses'} · ${_formatCredits(sumCredits)} credits',
                          style: typography.body.sm.copyWith(
                            color: colors.foreground,
                          ),
                        ),
                        const SizedBox(height: Space.md),
                        Text(
                          'PROJECTED CGPA',
                          style: typography.body.xs.copyWith(
                            fontWeight: FontWeight.w600,
                            letterSpacing: 0.6,
                            color: colors.mutedForeground,
                          ),
                        ),
                        if (projected == null)
                          Text(
                            'Add your current CGPA below',
                            style: typography.body.sm.copyWith(
                              color: colors.mutedForeground,
                            ),
                          )
                        else
                          Row(
                            children: [
                              Text(
                                currentCgpa!.value.toStringAsFixed(2),
                                style: typography.body.md.copyWith(
                                  color: colors.mutedForeground,
                                  fontFeatures: const [
                                    FontFeature.tabularFigures(),
                                  ],
                                ),
                              ),
                              Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: Space.xs,
                                ),
                                child: Icon(
                                  FLucideIcons.arrowRight,
                                  size: 14,
                                  color: colors.mutedForeground,
                                ),
                              ),
                              TweenAnimationBuilder<double>(
                                tween: Tween(end: projected),
                                duration: Motion.slow,
                                curve: Curves.easeOutCubic,
                                builder: (context, v, _) => Text(
                                  v.toStringAsFixed(2),
                                  style: typography.body.lg.copyWith(
                                    fontWeight: FontWeight.w600,
                                    color: colors.foreground,
                                    fontFeatures: const [
                                      FontFeature.tabularFigures(),
                                    ],
                                  ),
                                ),
                              ),
                              const SizedBox(width: Space.sm),
                              if (delta.abs() >= 0.005)
                                ToneBadge(
                                  label:
                                      '${delta > 0 ? '+' : '−'}${delta.abs().toStringAsFixed(2)}',
                                  tone: delta > 0
                                      ? colors.app.success
                                      : colors.app.danger,
                                ),
                            ],
                          ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.md),
              // Base values: prefilled from history, editable when needed.
              PressScale(
                scale: 0.99,
                onPress: () => editBase.value = !editBase.value,
                child: Row(
                  children: [
                    Icon(
                      FLucideIcons.history,
                      size: 14,
                      color: colors.mutedForeground,
                    ),
                    const SizedBox(width: Space.xs + 2),
                    Expanded(
                      child: Text(
                        currentCgpa == null
                            ? 'Set current CGPA and credits'
                            : 'Based on CGPA ${cgpaController.text} over ${earnedController.text} credits',
                        style: typography.body.xs.copyWith(
                          color: colors.mutedForeground,
                        ),
                      ),
                    ),
                    Text(
                      editBase.value ? 'Done' : 'Edit',
                      style: typography.body.xs.copyWith(
                        fontWeight: FontWeight.w600,
                        color: colors.app.accentTone.onSubtle,
                      ),
                    ),
                  ],
                ),
              ),
              AnimatedSize(
                duration: Motion.medium,
                curve: Curves.easeOutCubic,
                child: editBase.value || currentCgpa == null
                    ? Padding(
                        padding: const EdgeInsets.only(top: Space.md),
                        child: Row(
                          children: [
                            Expanded(
                              child: FTextField(
                                label: const Text('Current CGPA'),
                                hint: 'e.g. 8.35',
                                keyboardType:
                                    const TextInputType.numberWithOptions(
                                      decimal: true,
                                    ),
                                control: FTextFieldControl.managed(
                                  controller: cgpaController,
                                ),
                              ),
                            ),
                            const SizedBox(width: Space.sm + 2),
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
                      )
                    : const SizedBox(width: double.infinity),
              ),
            ],
          ),
        ),
        SectionHeader(
          title: 'Courses',
          trailing: FButton(
            variant: FButtonVariant.ghost,
            size: FButtonSizeVariant.sm,
            mainAxisSize: MainAxisSize.min,
            prefix: const Icon(FLucideIcons.calendarDays),
            onPress: timetable.value == null ? null : addCurrentSemester,
            child: const Text('Current semester'),
          ),
        ),
        AnimatedSize(
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
          alignment: Alignment.topCenter,
          child: Column(
            children: [
              for (final (i, row) in rows.value.indexed)
                Padding(
                  key: ValueKey(row.id),
                  padding: const EdgeInsets.only(bottom: Space.sm),
                  child: _CourseRowCard(
                    index: i,
                    row: row,
                    canRemove: rows.value.length > 1,
                    onCredits: (c) =>
                        updateRow(row.id, (r) => r.copyWith(credits: c)),
                    onGrade: (g) =>
                        updateRow(row.id, (r) => r.copyWith(grade: g)),
                    onRemove: () => removeRow(row.id),
                  ),
                ),
            ],
          ),
        ),
        FButton(
          variant: FButtonVariant.outline,
          prefix: const Icon(FLucideIcons.plus),
          onPress: addRow,
          child: const Text('Add course'),
        ),
      ],
    );
  }

  /// Nearest letter for a GPA, used only to colour the ring.
  static String _letterFor(double gpa) {
    if (gpa >= 9.5) return 'S';
    if (gpa >= 8.5) return 'A';
    if (gpa >= 7.5) return 'B';
    if (gpa >= 6.5) return 'C';
    if (gpa >= 5.5) return 'D';
    if (gpa >= 4.5) return 'E';
    return 'F';
  }
}

class _CourseRowCard extends StatelessWidget {
  const _CourseRowCard({
    required this.index,
    required this.row,
    required this.canRemove,
    required this.onCredits,
    required this.onGrade,
    required this.onRemove,
  });

  final int index;
  final _Row row;
  final bool canRemove;
  final ValueChanged<double> onCredits;
  final ValueChanged<String> onGrade;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final title = row.courseName.isNotEmpty
        ? row.courseName
        : 'Course ${index + 1}';

    return Surface(
      padding: const EdgeInsets.fromLTRB(
        Space.md + 2,
        Space.sm + 2,
        Space.sm,
        Space.md,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: typography.body.sm.copyWith(
                        fontWeight: FontWeight.w600,
                        color: colors.foreground,
                      ),
                    ),
                    if (row.courseCode.isNotEmpty)
                      Text(
                        row.courseCode,
                        style: typography.body.xs.copyWith(
                          color: colors.mutedForeground,
                        ),
                      ),
                  ],
                ),
              ),
              FButton.icon(
                variant: FButtonVariant.ghost,
                size: FButtonSizeVariant.sm,
                semanticsLabel: 'Fewer credits',
                onPress: row.credits > 0.5
                    ? () => onCredits((row.credits - 0.5).clamp(0.5, 30))
                    : null,
                child: const Icon(FLucideIcons.minus),
              ),
              SizedBox(
                width: 56,
                child: Text(
                  '${_formatCredits(row.credits)} cr',
                  textAlign: TextAlign.center,
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w500,
                    color: colors.foreground,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
              ),
              FButton.icon(
                variant: FButtonVariant.ghost,
                size: FButtonSizeVariant.sm,
                semanticsLabel: 'More credits',
                onPress: () => onCredits((row.credits + 0.5).clamp(0.5, 30)),
                child: const Icon(FLucideIcons.plus),
              ),
              if (canRemove)
                FButton.icon(
                  variant: FButtonVariant.ghost,
                  size: FButtonSizeVariant.sm,
                  semanticsLabel: 'Remove course',
                  onPress: onRemove,
                  child: Icon(FLucideIcons.x, color: colors.mutedForeground),
                ),
            ],
          ),
          const SizedBox(height: Space.sm),
          Padding(
            padding: const EdgeInsets.only(right: Space.xs + 2),
            child: Segmented<String>(
              value: row.grade,
              onChanged: onGrade,
              segments: [for (final g in _pickerGrades) (g, g)],
            ),
          ),
        ],
      ),
    );
  }
}
