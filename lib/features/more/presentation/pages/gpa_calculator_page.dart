import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/gpa_dial.dart';

const _gradePoints = {
  'S': 10,
  'A': 9,
  'B': 8,
  'C': 7,
  'D': 6,
  'E': 5,
  'F': 0,
  'N': 0,
};

class _Row {
  final int id;
  final int credits;
  final String grade;

  const _Row({required this.id, this.credits = 4, this.grade = 'A'});

  _Row copyWith({int? credits, String? grade}) => _Row(
    id: id,
    credits: credits ?? this.credits,
    grade: grade ?? this.grade,
  );
}

double _pointsFor(String grade) =>
    (_gradePoints[grade.trim().toUpperCase()] ?? 0).toDouble();

class GpaCalculatorPage extends HookConsumerWidget {
  const GpaCalculatorPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = useState(0);
    final rows = useState<List<_Row>>([const _Row(id: 1)]);
    final nextId = useState(2);
    final cgpaController = useTextEditingController();
    final earnedController = useTextEditingController();
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final history = ref.watch(gradeHistoryProvider);

    void addRow() {
      rows.value = [...rows.value, _Row(id: nextId.value)];
      nextId.value++;
    }

    void updateRow(int id, {required _Row Function(_Row) transform}) {
      rows.value = [
        for (final r in rows.value) if (r.id == id) transform(r) else r,
      ];
    }

    void removeRow(int id) {
      if (rows.value.length == 1) return;
      rows.value = rows.value.where((r) => r.id != id).toList();
    }

    void prefill() {
      final data = history.value;
      if (data == null || data.records.isEmpty) return;
      final seen = <String>{};
      final filled = <_Row>[];
      var uid = nextId.value + 1000;
      for (final rec in data.records) {
        if (!seen.add(rec.courseCode)) continue;
        final grade = rec.grade.trim().toUpperCase();
        if (!_gradePoints.containsKey(grade)) continue;
        filled.add(
          _Row(
            id: uid++,
            credits:
                int.tryParse(rec.credits.replaceAll(RegExp(r'[^0-9]'), '')) ??
                3,
            grade: grade,
          ),
        );
      }
      if (filled.isEmpty) return;
      rows.value = filled;
      nextId.value = uid;
      cgpaController.text = data.cgpa.cgpa.toString();
      earnedController.text = data.cgpa.creditsEarned.toString();
    }

    double sumCredits = 0;
    double weighted = 0;
    for (final r in rows.value) {
      sumCredits += r.credits;
      weighted += r.credits * _pointsFor(r.grade);
    }
    final semesterGpa = sumCredits > 0 ? weighted / sumCredits : 0.0;

    final currentCgpa =
        double.tryParse(cgpaController.text.replaceAll(',', '.')) ?? 0.0;
    final earned =
        double.tryParse(
          earnedController.text.replaceAll(RegExp(r'[^0-9.]'), ''),
        ) ??
        0.0;
    final projected =
        (earned + sumCredits) > 0
            ? (currentCgpa * earned + weighted) / (earned + sumCredits)
            : 0.0;
    final delta = projected - currentCgpa;

    return SingleChildScrollView(
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const EdgeInsets.fromLTRB(0, 8, 0, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(child: _modeButton(context, 'Semester GPA', 0, mode)),
              const SizedBox(width: 10),
              Expanded(child: _modeButton(context, 'What-if CGPA', 1, mode)),
            ],
          ),
          const SizedBox(height: 18),
          Center(
            child: AnimatedSwitcher(
              duration: const Duration(milliseconds: 300),
              child:
                  mode.value == 0
                      ? GpaDial(
                        key: const ValueKey('dial-gpa'),
                        value: semesterGpa,
                        label: 'SEMESTER GPA',
                        caption: '${sumCredits.round()} credits',
                      )
                      : GpaDial(
                        key: const ValueKey('dial-cgpa'),
                        value: projected,
                        label: 'PROJECTED CGPA',
                        caption:
                            delta.abs() < 0.005
                                ? 'no change'
                                : '${delta > 0 ? '+' : ''}${delta.toStringAsFixed(2)}',
                      ),
            ),
          ),
          if (mode.value == 1) ...[
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
                  final c =
                      up ? const Color(0xFF2E7D32) : const Color(0xFFD32F2F);
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
                            fontFeatures: const [
                              FontFeature.tabularFigures(),
                            ],
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
          if (mode.value == 1)
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
                      label: const Text('Credits earned'),
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
          Row(
            children: [
              Expanded(
                child: FTappable(
                  onPress: addRow,
                  child: Container(
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(10),
                      border: Border.all(
                        color: context.theme.colors.border,
                        width: 1.5,
                      ),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          FLucideIcons.plus,
                          size: 16,
                          color: context.theme.colors.primary,
                        ),
                        const SizedBox(width: 6),
                        Text(
                          'Add course',
                          style: TextStyle(
                            fontWeight: FontWeight.w600,
                            color: context.theme.colors.primary,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: FTappable(
                  onPress: prefill,
                  child: Container(
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(10),
                      border: Border.all(
                        color: context.theme.colors.border,
                        width: 1.5,
                      ),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          FLucideIcons.sparkles,
                          size: 16,
                          color: context.theme.colors.mutedForeground,
                        ),
                        const SizedBox(width: 6),
                        Text(
                          'From history',
                          style: TextStyle(
                            fontWeight: FontWeight.w600,
                            color: context.theme.colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _modeButton(
    BuildContext context,
    String label,
    int index,
    ValueNotifier<int> mode,
  ) {
    final selected = mode.value == index;
    return FTappable(
      onPress: () => mode.value = index,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOutCubic,
        padding: const EdgeInsets.symmetric(vertical: 10),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: selected
              ? context.theme.colors.primary
              : context.theme.colors.background,
          borderRadius: BorderRadius.circular(10),
          border: Border.all(color: context.theme.colors.border),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            fontSize: 13.5,
            color: selected
                ? context.theme.colors.primaryForeground
                : context.theme.colors.mutedForeground,
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
  final ValueChanged<int> onCredits;
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
    final points = _pointsFor(row.grade).toInt();
    final gradeColor =
        points >= 9
            ? const Color(0xFF2E7D32)
            : points >= 7
            ? const Color(0xFF1976D2)
            : points >= 5
            ? const Color(0xFFE65100)
            : const Color(0xFFD32F2F);

    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Container(
        padding: const EdgeInsets.fromLTRB(10, 8, 6, 8),
        decoration: BoxDecoration(
          color: context.theme.colors.primaryForeground,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: context.theme.colors.border),
        ),
        child: Row(
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
              onPress: () => onCredits((row.credits - 1).clamp(1, 30)),
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
                    '${row.credits}',
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
              onPress: () => onCredits((row.credits + 1).clamp(1, 30)),
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
                  for (final g in _gradePoints.keys) g: '$g (${_gradePoints[g]})',
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
      ),
    );
  }
}
