import 'dart:developer' show log;

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/providers/grade_history_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradeHistoryPage extends HookConsumerWidget {
  const GradeHistoryPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data = ref.watch(gradeHistoryProvider);

    Future<void> reload() async {
      try {
        await ref.read(gradeHistoryProvider.notifier).refresh();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    return RefreshIndicator(
      onRefresh: reload,
      displacement: 80,
      backgroundColor: context.theme.colors.primary,
      color: context.theme.colors.primaryForeground,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(8, 8, 8, 20),
        child: data.when(
          loading:
              () => SizedBox(
                height: MediaQuery.of(context).size.height * 0.7,
                child: const _CenterInfo(
                  title: "Loading grade history...",
                  icon: FIcons.loaderCircle,
                ),
              ),
          error:
              (e, _) => SizedBox(
                height: MediaQuery.of(context).size.height * 0.7,
                child: _CenterInfo(
                  title: "Unable to load grade history",
                  subtitle: commonErrorMessage(e),
                  icon: FIcons.triangleAlert,
                ),
              ),
          data: (gradeHistory) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _StudentCard(info: gradeHistory.student),
                const SizedBox(height: 10),
                _CgpaCard(cgpa: gradeHistory.cgpa),
                const SizedBox(height: 10),
                if (gradeHistory.records.isEmpty)
                  SizedBox(
                    height: MediaQuery.of(context).size.height * 0.4,
                    child: const _CenterInfo(
                      title: "No grade history found",
                      subtitle: "Pull to refresh and try again.",
                      icon: FIcons.fileX,
                    ),
                  )
                else
                  ...gradeHistory.records.map((r) => _HistoryCard(record: r)),
                if (gradeHistory.updateTime > BigInt.zero)
                  Center(
                    child: Padding(
                      padding: const EdgeInsets.only(top: 12),
                      child: Text(
                        "Data updated on ${formatUnixTimestamp(gradeHistory.updateTime.toInt())}",
                        style: TextStyle(
                          fontSize: 13,
                          color: context.theme.colors.mutedForeground,
                          fontStyle: FontStyle.italic,
                        ),
                      ),
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _StudentCard extends ConsumerWidget {
  final GradeHistoryStudentInfo info;
  const _StudentCard({required this.info});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        gradient:
            darkMode
                ? null
                : const LinearGradient(
                  colors: [
                    MarksColors.theoryCardBackground,
                    MarksColors.theoryCardSecondary,
                  ],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
        borderRadius: BorderRadius.circular(12),
        color: darkMode ? context.theme.colors.primaryForeground : null,
        boxShadow: const [
          BoxShadow(
            color: MarksColors.cardShadow,
            blurRadius: 10,
            offset: Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            info.name,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w700,
              color:
                  darkMode
                      ? context.theme.colors.primary
                      : MarksColors.primaryText,
            ),
          ),
          const SizedBox(height: 6),
          Text(
            info.regNo,
            style: TextStyle(
              fontSize: 13,
              color:
                  darkMode
                      ? context.theme.colors.mutedForeground
                      : MarksColors.secondaryText,
            ),
          ),
          const SizedBox(height: 3),
          Text(
            info.programmeBranch,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontSize: 13,
              color:
                  darkMode
                      ? context.theme.colors.mutedForeground
                      : MarksColors.secondaryText,
            ),
          ),
        ],
      ),
    );
  }
}

class _CgpaCard extends StatelessWidget {
  final GradeHistoryCgpa cgpa;
  const _CgpaCard({required this.cgpa});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: context.theme.colors.primaryForeground,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            "CGPA Summary",
            style: TextStyle(
              fontWeight: FontWeight.w700,
              fontSize: 15,
              color: context.theme.colors.primary,
            ),
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _chip(context, "CGPA", cgpa.cgpa),
              _chip(context, "Credits Reg", cgpa.creditsRegistered),
              _chip(context, "Credits Earned", cgpa.creditsEarned),
              _chip(context, "S", cgpa.sGrades),
              _chip(context, "A", cgpa.aGrades),
              _chip(context, "B", cgpa.bGrades),
              _chip(context, "C", cgpa.cGrades),
              _chip(context, "D", cgpa.dGrades),
              _chip(context, "E", cgpa.eGrades),
              _chip(context, "F", cgpa.fGrades),
              _chip(context, "N", cgpa.nGrades),
            ],
          ),
        ],
      ),
    );
  }

  Widget _chip(BuildContext context, String k, String v) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Text(
        "$k: $v",
        style: TextStyle(
          color: context.theme.colors.primary,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}

class _HistoryCard extends ConsumerStatefulWidget {
  final GradeHistoryRecord record;
  const _HistoryCard({required this.record});

  @override
  ConsumerState<_HistoryCard> createState() => _HistoryCardState();
}

class _HistoryCardState extends ConsumerState<_HistoryCard>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _animation;
  bool _expanded = false;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 250),
    );
    _animation = CurvedAnimation(parent: _controller, curve: Curves.easeInOut);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Color _gradeColor(String grade) {
    switch (grade.trim().toUpperCase()) {
      case 'S':
      case 'A':
        return MarksColors.excellentColor;
      case 'B':
      case 'C':
        return MarksColors.averageColor;
      case 'F':
      case 'N':
        return MarksColors.failedText;
      default:
        return MarksColors.secondaryText;
    }
  }

  void _toggle() {
    setState(() => _expanded = !_expanded);
    if (_expanded) {
      _controller.forward();
    } else {
      _controller.reverse();
    }
  }

  @override
  Widget build(BuildContext context) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final r = widget.record;

    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        children: [
          FTappable(
            onPress: _toggle,
            child: Container(
              decoration: BoxDecoration(
                gradient:
                    !darkMode
                        ? const LinearGradient(
                          colors: [
                            MarksColors.theoryCardBackground,
                            MarksColors.theoryCardSecondary,
                          ],
                          begin: Alignment.topLeft,
                          end: Alignment.bottomRight,
                        )
                        : null,
                borderRadius: BorderRadius.circular(12),
                color: darkMode ? context.theme.colors.primaryForeground : null,
                boxShadow: const [
                  BoxShadow(
                    color: MarksColors.cardShadow,
                    blurRadius: 10,
                    offset: Offset(0, 4),
                  ),
                ],
              ),
              child: Padding(
                padding: const EdgeInsets.all(14),
                child: Column(
                  children: [
                    Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: context.theme.colors.primaryForeground,
                            borderRadius: BorderRadius.circular(10),
                          ),
                          child: const Icon(
                            FIcons.bookOpen,
                            color: MarksColors.theoryIcon,
                            size: 20,
                          ),
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                r.courseTitle,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  fontWeight: FontWeight.w700,
                                  fontSize: 16,
                                  color:
                                      darkMode
                                          ? context.theme.colors.primary
                                          : MarksColors.primaryText,
                                ),
                              ),
                              const SizedBox(height: 2),
                              Text(
                                "${r.courseCode} • ${r.courseType} • ${r.credits} credits",
                                style: TextStyle(
                                  fontSize: 12,
                                  color:
                                      darkMode
                                          ? context.theme.colors.primary
                                          : MarksColors.secondaryText,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 10,
                            vertical: 7,
                          ),
                          decoration: BoxDecoration(
                            color: context.theme.colors.primaryForeground,
                            borderRadius: BorderRadius.circular(20),
                            border: Border.all(
                              color: _gradeColor(r.grade).withValues(alpha: .4),
                            ),
                          ),
                          child: Text(
                            r.grade,
                            style: TextStyle(
                              fontWeight: FontWeight.w700,
                              color: _gradeColor(r.grade),
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 10),
                    Row(
                      children: [
                        Expanded(
                          child: _metaChip(
                            context,
                            "Exam",
                            r.examMonth,
                            icon: FIcons.calendarDays,
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: _metaChip(
                            context,
                            "Declared",
                            r.resultDeclared,
                            icon: FIcons.calendarCheck,
                          ),
                        ),
                      ],
                    ),
                    if (r.courseDistribution.trim().isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 8),
                        child: Align(
                          alignment: Alignment.centerLeft,
                          child: _metaChip(
                            context,
                            "Distribution",
                            r.courseDistribution,
                            icon: FIcons.bookOpen,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
          SizeTransition(
            sizeFactor: _animation,
            child: _HistoryDetails(record: r),
          ),
        ],
      ),
    );
  }

  Widget _metaChip(
    BuildContext context,
    String label,
    String value, {
    required IconData icon,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      decoration: BoxDecoration(
        color: context.theme.colors.primaryForeground,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Row(
        children: [
          Icon(icon, size: 14, color: context.theme.colors.mutedForeground),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              "$label: $value",
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 12,
                color: context.theme.colors.primary,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _HistoryDetails extends StatelessWidget {
  final GradeHistoryRecord record;
  const _HistoryDetails({required this.record});

  @override
  Widget build(BuildContext context) {
    if (record.attempts.isEmpty) {
      return Container(
        width: double.infinity,
        margin: const EdgeInsets.only(top: 8),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: context.theme.colors.primaryForeground,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: context.theme.colors.border),
        ),
        child: Text(
          "No attempt breakdown available",
          style: TextStyle(
            color: context.theme.colors.mutedForeground,
            fontStyle: FontStyle.italic,
          ),
        ),
      );
    }

    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(top: 8),
      decoration: BoxDecoration(
        color: context.theme.colors.primaryForeground,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Column(
        children: [
          for (int i = 0; i < record.attempts.length; i++) ...[
            _AttemptItem(attempt: record.attempts[i]),
            if (i != record.attempts.length - 1)
              Divider(
                height: 1,
                thickness: 1,
                color: context.theme.colors.border,
              ),
          ],
        ],
      ),
    );
  }
}

class _AttemptItem extends StatelessWidget {
  final GradeHistoryAttempt attempt;
  const _AttemptItem({required this.attempt});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            attempt.courseTitle,
            style: TextStyle(
              fontSize: 15,
              fontWeight: FontWeight.w700,
              color: context.theme.colors.primary,
            ),
          ),
          const SizedBox(height: 6),
          Wrap(
            spacing: 10,
            runSpacing: 8,
            children: [
              _small(context, "Code", attempt.courseCode),
              _small(context, "Type", attempt.courseType),
              _small(context, "Credits", attempt.credits),
              _small(context, "Grade", attempt.grade),
              _small(context, "Exam", attempt.examMonth),
              _small(context, "Declared", attempt.resultDeclared),
            ],
          ),
        ],
      ),
    );
  }

  Widget _small(BuildContext context, String k, String v) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 6),
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Text(
        "$k: $v",
        style: TextStyle(
          fontSize: 12,
          color: context.theme.colors.primary,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}

class _CenterInfo extends StatelessWidget {
  final String title;
  final String? subtitle;
  final IconData icon;

  const _CenterInfo({required this.title, required this.icon, this.subtitle});

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Icon(icon, size: 28, color: context.theme.colors.mutedForeground),
        const SizedBox(height: 10),
        Text(
          title,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            color: context.theme.colors.primary,
          ),
        ),
        if (subtitle != null) ...[
          const SizedBox(height: 6),
          Text(
            subtitle!,
            textAlign: TextAlign.center,
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
        ],
      ],
    );
  }
}
