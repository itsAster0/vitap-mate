import 'dart:developer' show log;
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/theme.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/marks_card.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';

class MarksPage extends HookConsumerWidget {
  const MarksPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref.read(marksProvider.notifier).updatemarks();
        } catch (e, _) {
          ();
        }
      });
      return null;
    }, []);
    Future<void> update() async {
      try {
        await ref.read(marksProvider.notifier).updatemarks();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }
   final darkMode = ref.watch(themeProvider)==ThemeMode.dark;

    var marksData = ref.watch(marksProvider);

    return RefreshIndicator(
      onRefresh: update,
      backgroundColor: MarksColors.tableBackground,
      color: darkMode? context.theme.colors.primaryForeground : MarksColors.primaryText,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        child: ConstrainedBox(
          constraints: BoxConstraints(
            minHeight: MediaQuery.of(context).size.height * 0.8,
          ),
          child: marksData.when(
            data: (data) {
              if (data.records.isEmpty) {
                return _buildEmptyState(context);
              }

              return Padding(
                padding: const EdgeInsets.all(6),
                child: Column(
                  spacing: 4,
                  children: [
                    for (final record in data.records)
                      MarksCard(record: record),

                    _buildFooter(data.updateTime.toInt()),
                  ],
                ),
              );
            },
            error: (e, se) => _buildErrorState(e,context),
            loading: () => _buildLoadingState(context),
          ),
        ),
      ),
    );
  }

  Widget _buildEmptyState(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: context.theme.colors.primaryForeground,
              borderRadius: BorderRadius.circular(16),
            ),
            child: Icon(
              Icons.assignment_outlined,
              size: 48,
              color: context.theme.colors.primary,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            "No marks data available",
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: context.theme.colors.primary,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            "Check back later for updates",
            style: TextStyle(fontSize: 14, color: context.theme.colors.primary),
          ),
        ],
      ),
    );
  }

  Widget _buildErrorState(Object error,BuildContext context) {
    String msg = commonErrorMessage(error);
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: MarksColors.failedBackground,
              borderRadius: BorderRadius.circular(16),
            ),
            child: Icon(
              Icons.error_outline_rounded,
              size: 48,
              color: MarksColors.failedText,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            "Unable to load marks data",
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: context.theme.colors.primary,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            msg,
            style: TextStyle(fontSize: 14, color: context.theme.colors.primary),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }

  Widget _buildLoadingState(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          SizedBox(
            width: 50,
            height: 50,
            child: CircularProgressIndicator(
              color: MarksColors.theoryIcon,
              strokeWidth: 3,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            "Loading marks data...",
            style: TextStyle(
              fontSize: 14,
              color: context.theme.colors.primary,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildFooter(int updateTime) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.only(top: 12, bottom: 20),
        child: Text(
          "Data updated on ${formatUnixTimestamp(updateTime)}",
          style: TextStyle(
            fontSize: 14,
            color: MarksColors.tertiaryText,
            fontStyle: FontStyle.italic,
          ),
        ),
      ),
    );
  }
}
