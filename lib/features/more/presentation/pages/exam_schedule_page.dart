import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/theme.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/more/presentation/widgets/exam_schedule_card.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';

class ExamSchedulePage extends HookConsumerWidget {
  const ExamSchedulePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final autoRefreshOnOpen = ref.watch(autoRefreshOnOpenProvider);
    Future<void> update() async {
      try {
        await ref.read(examScheduleProvider.notifier).updatexamschedule();
      } catch (e) {
        if (context.mounted) disCommonToast(context, e);
      }
    }

    final darkMode = ref.watch(themeControllerProvider) == ThemeMode.dark;

    useEffect(() {
      if (!autoRefreshOnOpen) {
        return null;
      }
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref.read(examScheduleProvider.notifier).updatexamschedule();
        } catch (e, _) {
          ();
        }
      });

      return null;
    }, [autoRefreshOnOpen]);
    var examData = ref.watch(examScheduleProvider);
    return RefreshIndicator(
      onRefresh: () async {
        await update();
      },
      backgroundColor:
          darkMode
              ? context.theme.colors.primaryForeground
              : ExamColors.tableBackground,
      color: darkMode ? context.theme.colors.primary : ExamColors.primaryText,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        child: Container(
          color: context.theme.colors.background,
          child: ConstrainedBox(
            constraints: BoxConstraints(
              minHeight: MediaQuery.of(context).size.height * 0.8,
            ),
            child: examData.when(
              data: (data) {
                if (data.exams.isEmpty) {
                  return Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Container(
                          padding: const EdgeInsets.all(24),
                          decoration: BoxDecoration(
                            color: ExamColors.tableRowAlternate,
                            borderRadius: BorderRadius.circular(16),
                          ),
                          child: Icon(
                            Icons.event_note_outlined,
                            size: 48,
                            color:
                                darkMode
                                    ? context.theme.colors.primary
                                    : ExamColors.tertiaryText,
                          ),
                        ),
                        const SizedBox(height: 16),
                        Text(
                          "No exam schedule available",
                          style: TextStyle(
                            fontSize: 16,
                            fontWeight: FontWeight.w600,
                            color:
                                darkMode
                                    ? context.theme.colors.primary
                                    : ExamColors.secondaryText,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          "Check back later for updates",
                          style: TextStyle(
                            fontSize: 14,
                            color:
                                darkMode
                                    ? context.theme.colors.primary
                                    : ExamColors.tertiaryText,
                          ),
                        ),
                      ],
                    ),
                  );
                }
                return Padding(
                  padding: const EdgeInsets.all(6),
                  child: Column(
                    spacing: 4,
                    children: [
                      for (final i in data.exams) ExamScheduleCard(record: i),
                      Center(
                        child: Padding(
                          padding: const EdgeInsets.only(top: 12, bottom: 20),
                          child: Text(
                            "Data updated on ${formatUnixTimestamp(data.updateTime.toInt())}",
                            style: TextStyle(
                              fontSize: 14,
                              color: ExamColors.tertiaryText,
                              fontStyle: FontStyle.italic,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                );
              },
              error: (e, se) {
                String msg = commonErrorMessage(e);
                return Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Container(
                        padding: const EdgeInsets.all(24),
                        decoration: BoxDecoration(
                          color: ExamColors.completedBackground,
                          borderRadius: BorderRadius.circular(16),
                        ),
                        child: Icon(
                          Icons.error_outline_rounded,
                          size: 48,
                          color: ExamColors.completedText,
                        ),
                      ),
                      const SizedBox(height: 16),
                      Text(
                        "Unable to load exam schedule",
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                          color:
                              darkMode
                                  ? context.theme.colors.primary
                                  : ExamColors.secondaryText,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Text(
                        msg,
                        style: TextStyle(
                          fontSize: 14,
                          color:
                              darkMode
                                  ? context.theme.colors.primary
                                  : ExamColors.tertiaryText,
                        ),
                        textAlign: TextAlign.center,
                      ),
                    ],
                  ),
                );
              },
              loading: () {
                return Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 280),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            "Loading exam schedule...",
                            style: TextStyle(
                              fontSize: 14,
                              color:
                                  darkMode
                                      ? context.theme.colors.primary
                                      : ExamColors.secondaryText,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          const SizedBox(height: 12),
                          LinearProgressIndicator(
                            minHeight: 3,
                            color: ExamColors.examIcon,
                            backgroundColor: ExamColors.examIcon.withValues(
                              alpha: 0.18,
                            ),
                            borderRadius: BorderRadius.circular(8),
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
        ),
      ),
    );
  }
}
