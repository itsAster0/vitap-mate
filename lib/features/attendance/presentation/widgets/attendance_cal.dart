import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance_colors.dart';

class AttendanceCalculator extends HookConsumerWidget {
  final int currentAttended;
  final int currentTotal;

  const AttendanceCalculator({
    super.key,
    required this.currentAttended,
    required this.currentTotal,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    (int, int) getDefaultFuturePlan(int attended, int total) {
      if (total <= 0) return (0, 0);
      final currentPct = (attended / total) * 100;
      if (currentPct >= 75) {
        final maxSkip = ((4 * attended - 3 * total) / 3).floor();
        return (0, maxSkip > 0 ? maxSkip : 0);
      }
      final needAttend = 3 * total - 4 * attended;
      return (needAttend > 0 ? needAttend : 0, 0);
    }

    final initialPlan = useMemoized(
      () => getDefaultFuturePlan(currentAttended, currentTotal),
      [currentAttended, currentTotal],
    );
    final attended = useState(currentAttended);
    final total = useState(currentTotal);
    final editCurrent = useState(false);
    final futureAttend = useState(initialPlan.$1);
    final futureSkip = useState(initialPlan.$2);

    useEffect(() {
      final defaultPlan = getDefaultFuturePlan(attended.value, total.value);
      futureAttend.value = defaultPlan.$1;
      futureSkip.value = defaultPlan.$2;
      return null;
    }, [attended.value, total.value]);

    double getCurrentPercentage() {
      if (total.value == 0) return 0.0;
      return (attended.value / total.value) * 100;
    }

    double getPredictedPercentage() {
      final newTotal = total.value + futureAttend.value + futureSkip.value;
      final newAttended = attended.value + futureAttend.value;
      if (newTotal == 0) return 0.0;
      return (newAttended / newTotal) * 100;
    }

    Color getPercentageColor(double percentage) {
      if (percentage >= 75) return AttendanceColors.presentText;
      if (percentage >= 65) return Colors.orange;
      return AttendanceColors.absentText;
    }

    return Container(
      height: double.infinity,
      width: double.infinity,
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      ),
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              "Attendance Calculator",
              style: TextStyle(
                fontSize: 20,
                fontWeight: FontWeight.w700,
                color:
                    darkMode
                        ? context.theme.colors.primary
                        : AttendanceColors.primaryText,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              "Plan your attendance strategy",
              style: TextStyle(
                fontSize: 14,
                color: AttendanceColors.tertiaryText,
              ),
            ),
            const SizedBox(height: 24),

            Container(
              padding: const EdgeInsets.all(20),
              decoration: BoxDecoration(
                color:
                    darkMode
                        ? context.theme.colors.primaryForeground
                        : AttendanceColors.tableBackground,
                borderRadius: BorderRadius.circular(16),
                border: Border.all(
                  color: context.theme.colors.border,
                  width: 1,
                ),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Column(
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: Row(
                              children: [
                                Icon(
                                  Icons.analytics_outlined,
                                  color: AttendanceColors.theoryIcon,
                                  size: 20,
                                ),
                                const SizedBox(width: 8),
                                Text(
                                  "Current Attendance",
                                  style: TextStyle(
                                    fontSize: 16,
                                    fontWeight: FontWeight.w600,
                                    color:
                                        darkMode
                                            ? context.theme.colors.primary
                                            : AttendanceColors.primaryText,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          Text("Edit"),
                          FCheckbox(
                            value: editCurrent.value,
                            onChange: (value) => editCurrent.value = value,
                          ),
                        ],
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),

                  Row(
                    children: [
                      Expanded(
                        child: _buildInputCard(
                          enable: editCurrent.value,
                          context: context,
                          darkMode: darkMode,
                          label: "Attended",
                          value: attended.value,
                          onIncrement: () {
                            attended.value++;
                            total.value++;
                          },
                          onDecrement: () {
                            if (attended.value > 0) {
                              attended.value--;
                              total.value--;
                            }
                          },
                          color: AttendanceColors.presentText,
                          icon: Icons.check_circle_outline,
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: _buildInputCard(
                          enable: editCurrent.value,
                          context: context,
                          darkMode: darkMode,
                          label: "Skipped",
                          value: total.value - attended.value,
                          onIncrement: () => total.value++,
                          onDecrement: () {
                            if (total.value > attended.value) total.value--;
                          },
                          color: AttendanceColors.absentText,
                          icon: Icons.cancel_outlined,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),

                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color:
                          darkMode
                              ? context.theme.colors.background
                              : Colors.white,
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(
                        color: context.theme.colors.border,
                        width: 1,
                      ),
                    ),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              "Total Classes",
                              style: TextStyle(
                                fontSize: 12,
                                color: AttendanceColors.tertiaryText,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              "${total.value}",
                              style: TextStyle(
                                fontSize: 24,
                                fontWeight: FontWeight.w700,
                                color:
                                    darkMode
                                        ? context.theme.colors.primary
                                        : AttendanceColors.primaryText,
                              ),
                            ),
                          ],
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 16,
                            vertical: 8,
                          ),
                          decoration: BoxDecoration(
                            color: getPercentageColor(
                              getCurrentPercentage(),
                            ).withValues(alpha: 0.1),
                            borderRadius: BorderRadius.circular(20),
                            border: Border.all(
                              color: getPercentageColor(getCurrentPercentage()),
                              width: 2,
                            ),
                          ),
                          child: Text(
                            "${getCurrentPercentage().toStringAsFixed(1)}%",
                            style: TextStyle(
                              fontSize: 20,
                              fontWeight: FontWeight.w700,
                              color: getPercentageColor(getCurrentPercentage()),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),

            const SizedBox(height: 24),

            Container(
              padding: const EdgeInsets.all(20),
              decoration: BoxDecoration(
                color:
                    darkMode
                        ? context.theme.colors.primaryForeground
                        : AttendanceColors.tableBackground,
                borderRadius: BorderRadius.circular(16),
                border: Border.all(
                  color: context.theme.colors.border,
                  width: 1,
                ),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Row(
                          children: [
                            Icon(
                              Icons.calculate_outlined,
                              color: Colors.purple,
                              size: 20,
                            ),
                            const SizedBox(width: 8),
                            Text(
                              "Future Prediction",
                              style: TextStyle(
                                fontSize: 16,
                                fontWeight: FontWeight.w600,
                                color:
                                    darkMode
                                        ? context.theme.colors.primary
                                        : AttendanceColors.primaryText,
                              ),
                            ),
                          ],
                        ),
                      ),
                      FButton(
                        style: FButtonStyle.outline(),
                        onPress: () {
                          futureAttend.value = 0;
                          futureSkip.value = 0;
                        },
                        child: const Text("Reset"),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),

                  Row(
                    children: [
                      Expanded(
                        child: _buildInputCard(
                          context: context,
                          darkMode: darkMode,
                          label: "Will Attend",
                          value: futureAttend.value,
                          onIncrement: () => futureAttend.value++,
                          onDecrement: () {
                            if (futureAttend.value > 0) futureAttend.value--;
                          },
                          color: AttendanceColors.presentText,
                          icon: Icons.add_circle_outline,
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: _buildInputCard(
                          context: context,
                          darkMode: darkMode,
                          label: "Will Skip",
                          value: futureSkip.value,
                          onIncrement: () => futureSkip.value++,
                          onDecrement: () {
                            if (futureSkip.value > 0) futureSkip.value--;
                          },
                          color: AttendanceColors.absentText,
                          icon: Icons.remove_circle_outline,
                        ),
                      ),
                    ],
                  ),

                  if (futureAttend.value > 0 || futureSkip.value > 0) ...[
                    const SizedBox(height: 16),

                    Container(
                      padding: const EdgeInsets.all(16),
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(
                          color: getPercentageColor(getPredictedPercentage()),
                          width: 2,
                        ),
                      ),
                      child: Column(
                        children: [
                          Text(
                            "Predicted Attendance",
                            style: TextStyle(
                              fontSize: 12,
                              fontWeight: FontWeight.w600,
                              color: AttendanceColors.tertiaryText,
                            ),
                          ),
                          const SizedBox(height: 8),
                          Text(
                            "${getPredictedPercentage().toStringAsFixed(1)}%",
                            style: TextStyle(
                              fontSize: 24,
                              fontWeight: FontWeight.w700,
                              color: getPercentageColor(
                                getPredictedPercentage(),
                              ),
                            ),
                          ),
                          const SizedBox(height: 8),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Icon(
                                getPredictedPercentage() >=
                                        getCurrentPercentage()
                                    ? Icons.trending_up
                                    : Icons.trending_down,
                                color: getPercentageColor(
                                  getPredictedPercentage(),
                                ),
                                size: 16,
                              ),
                              const SizedBox(width: 4),
                              Text(
                                "${(getPredictedPercentage() - getCurrentPercentage()).abs().toStringAsFixed(1)}%",
                                style: TextStyle(
                                  fontSize: 14,
                                  fontWeight: FontWeight.w600,
                                  color: AttendanceColors.tertiaryText,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 12),
                          Text(
                            "${attended.value + futureAttend.value} / ${total.value + futureAttend.value + futureSkip.value} classes",
                            style: TextStyle(
                              fontSize: 13,
                              color: AttendanceColors.secondaryText,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildInputCard({
    bool enable = true,
    required BuildContext context,
    required bool darkMode,
    required String label,
    required int value,
    required VoidCallback onIncrement,
    required VoidCallback onDecrement,
    required Color color,
    required IconData icon,
  }) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: darkMode ? context.theme.colors.background : Colors.white,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: context.theme.colors.border, width: 1),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, color: color, size: 16),
              const SizedBox(width: 6),
              Text(
                label,
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w500,
                  color: AttendanceColors.tertiaryText,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              if (enable)
                FTappable(
                  onPress: onDecrement,
                  child: Container(
                    padding: const EdgeInsets.all(6),
                    decoration: BoxDecoration(
                      color: color.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: color.withValues(alpha: 0.3)),
                    ),
                    child: Icon(Icons.remove, size: 16, color: color),
                  ),
                ),
              Text(
                "$value",
                style: TextStyle(
                  fontSize: 24,
                  fontWeight: FontWeight.w700,
                  color:
                      darkMode
                          ? context.theme.colors.primary
                          : AttendanceColors.primaryText,
                ),
              ),
              if (enable)
                FTappable(
                  onPress: onIncrement,
                  child: Container(
                    padding: const EdgeInsets.all(6),
                    decoration: BoxDecoration(
                      color: color.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: color.withValues(alpha: 0.3)),
                    ),
                    child: Icon(Icons.add, size: 16, color: color),
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }
}
