import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/more/presentation/widgets/marks_table.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksCard extends HookConsumerWidget {
  final MarksRecord record;

  const MarksCard({super.key, required this.record});

  (
    double totalWeightageGained,
    double totalWeightagePossible,
    double weightagePercentage,
  )
  _calculateWeightageTotals() {
    double totalWeightageGained = 0;
    double totalWeightagePossible = 0;

    for (final mark in record.marks) {
      final weightageGained = double.tryParse(mark.weightagemark) ?? 0;
      totalWeightageGained += weightageGained;

      final weightagePossible =
          double.tryParse(mark.weightage.replaceAll('%', '')) ?? 0;
      totalWeightagePossible += weightagePossible;
    }

    final weightagePercentage =
        totalWeightagePossible > 0
            ? (totalWeightageGained / totalWeightagePossible) * 100
            : 0.0;
    return (totalWeightageGained, totalWeightagePossible, weightagePercentage);
  }

  Color _getPerformanceColor(double percentage) {
    if (percentage >= 85) return MarksColors.excellentColor;
    if (percentage >= 75) return MarksColors.veryGoodColor;
    if (percentage >= 65) return MarksColors.goodColor;
    if (percentage >= 55) return MarksColors.satisfactoryColor;
    return MarksColors.averageColor;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = useAnimationController(
      duration: const Duration(milliseconds: 300),
    );
    final animation = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeInOut),
      [controller],
    );

    final expanded = useState(false);
    final totals = _calculateWeightageTotals();

    void toggle() {
      expanded.value = !expanded.value;
      if (expanded.value) {
        controller.forward();
      } else {
        controller.reverse();
      }
    }

    final isLab = record.coursetype.toLowerCase().endsWith("lab");

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          FTappable(
            onPress: toggle,
            child: Container(
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  colors:
                      isLab
                          ? [
                            MarksColors.labCardBackground,
                            MarksColors.labCardSecondary,
                          ]
                          : [
                            MarksColors.theoryCardBackground,
                            MarksColors.theoryCardSecondary,
                          ],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
                borderRadius: BorderRadius.circular(12),
                boxShadow: [
                  BoxShadow(
                    color: MarksColors.cardShadow,
                    blurRadius: 10,
                    offset: const Offset(0, 4),
                  ),
                ],
                border: Border.all(
                  color: Colors.white.withValues(alpha: .8),
                  width: 1,
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  spacing: 12,
                  children: [
                    Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: Colors.white.withValues(alpha: 0.9),
                            borderRadius: BorderRadius.circular(10),
                            boxShadow: [
                              BoxShadow(
                                color: (isLab
                                        ? MarksColors.labIcon
                                        : MarksColors.theoryIcon)
                                    .withValues(alpha: 0.1),
                                blurRadius: 8,
                                offset: const Offset(0, 2),
                              ),
                            ],
                          ),
                          child: Icon(
                            isLab ? FIcons.flaskConical : FIcons.libraryBig,
                            color:
                                isLab
                                    ? MarksColors.labIcon
                                    : MarksColors.theoryIcon,
                            size: 20,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                record.coursetitle,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(
                                  fontWeight: FontWeight.w700,
                                  fontSize: 16,
                                  color: MarksColors.primaryText,
                                  height: 1.3,
                                ),
                              ),
                              const SizedBox(height: 4),
                              Text(
                                record.coursecode,
                                style: const TextStyle(
                                  fontSize: 12,
                                  fontWeight: FontWeight.w600,
                                  color: MarksColors.secondaryText,
                                  letterSpacing: 0.5,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),

                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 8,
                          ),
                          decoration: BoxDecoration(
                            color: Colors.white.withValues(alpha: 0.9),
                            borderRadius: BorderRadius.circular(20),
                            border: Border.all(
                              color: _getPerformanceColor(
                                totals.$3,
                              ).withValues(alpha: 0.4),
                              width: 1.5,
                            ),
                          ),
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Text(
                                "${totals.$1.toStringAsFixed(1)}/${totals.$2.toStringAsFixed(1)}",
                                style: const TextStyle(
                                  fontSize: 12,
                                  fontWeight: FontWeight.w700,
                                  color: MarksColors.primaryText,
                                ),
                              ),
                              Text(
                                "${totals.$3.toStringAsFixed(1)}%",
                                style: TextStyle(
                                  fontSize: 14,
                                  fontWeight: FontWeight.w800,
                                  color: _getPerformanceColor(totals.$3),
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),
                        AnimatedRotation(
                          turns: expanded.value ? 0.5 : 0,
                          duration: const Duration(milliseconds: 300),
                          child: Icon(
                            Icons.keyboard_arrow_down,
                            color: MarksColors.secondaryText,
                            size: 24,
                          ),
                        ),
                      ],
                    ),

                    Container(
                      height: 8,
                      decoration: BoxDecoration(
                        color: Colors.white.withValues(alpha: 0.4),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: FractionallySizedBox(
                        alignment: Alignment.centerLeft,
                        widthFactor:
                            totals.$2 > 0
                                ? (totals.$1 / totals.$2).clamp(0.0, 1.0)
                                : 0,
                        child: Container(
                          decoration: BoxDecoration(
                            gradient: LinearGradient(
                              colors: [
                                _getPerformanceColor(
                                  totals.$3,
                                ).withValues(alpha: 0.8),
                                _getPerformanceColor(totals.$3),
                              ],
                            ),
                            borderRadius: BorderRadius.circular(4),
                            boxShadow: [
                              BoxShadow(
                                color: _getPerformanceColor(
                                  totals.$3,
                                ).withValues(alpha: 0.3),
                                blurRadius: 4,
                                offset: const Offset(0, 1),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                    Row(
                      children: [
                        Icon(
                          Icons.person_outline,
                          size: 16,
                          color: MarksColors.secondaryText,
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            "by ${record.faculity}",
                            style: const TextStyle(
                              fontSize: 14,
                              color: MarksColors.secondaryText,
                              fontWeight: FontWeight.w500,
                            ),
                          ),
                        ),

                        const SizedBox(width: 8),
                        Text(
                          "${record.marks.length} items",
                          style: const TextStyle(
                            fontSize: 12,
                            color: MarksColors.tertiaryText,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),
          AnimatedBuilder(
            animation: animation,
            builder:
                (context, child) =>
                    FCollapsible(value: animation.value, child: child!),
            child: Container(
              margin: const EdgeInsets.symmetric(horizontal: 4),
              child: MarksTable(marks: record.marks),
            ),
          ),
        ],
      ),
    );
  }
}
