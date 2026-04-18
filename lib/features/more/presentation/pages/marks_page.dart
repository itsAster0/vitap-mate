import 'dart:developer' show log;
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/marks_card.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksPage extends HookConsumerWidget {
  const MarksPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final autoRefresh = ref.watch(autoRefreshProvider);
    useEffect(() {
      if (!autoRefresh) return null;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        ref.read(marksProvider.notifier).updatemarks().catchError((e, st) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });
      return null;
    }, [autoRefresh]);
    Future<void> update() async {
      try {
        await ref.read(marksProvider.notifier).updatemarks();
      } catch (e) {
        log("$e");
      }
    }

    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;

    var marksData = ref.watch(marksProvider);

    return Container(
      color: context.theme.colors.background,
      child: marksData.when(
        data: (data) => _MarksFilterView(
          records: data.records,
          updateTime: data.updateTime.toInt(),
          onRefresh: update,
          refreshIndicatorColor: darkMode
              ? context.theme.colors.primaryForeground
              : MarksColors.primaryText,
          emptyStateBuilder: _buildEmptyState,
          footerBuilder: _buildFooter,
        ),
        error: (e, se) => _buildErrorState(e, context),
        loading: () => _buildLoadingState(context),
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

  Widget _buildErrorState(Object error, BuildContext context) {
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

enum _CourseFilter { all, theory, lab }

class _MarksFilterView extends HookWidget {
  final List<MarksRecord> records;
  final int updateTime;
  final RefreshCallback onRefresh;
  final Color refreshIndicatorColor;
  final Widget Function(BuildContext context) emptyStateBuilder;
  final Widget Function(int updateTime) footerBuilder;

  const _MarksFilterView({
    required this.records,
    required this.updateTime,
    required this.onRefresh,
    required this.refreshIndicatorColor,
    required this.emptyStateBuilder,
    required this.footerBuilder,
  });

  @override
  Widget build(BuildContext context) {
    final selected = useState(_CourseFilter.all);
    final theoryRecords = records.where((record) => !record.islab()).toList();
    final labRecords = records.where((record) => record.islab()).toList();
    final filteredRecords = switch (selected.value) {
      _CourseFilter.all => records,
      _CourseFilter.theory => theoryRecords,
      _CourseFilter.lab => labRecords,
    };
    final emptyBuilder = switch (selected.value) {
      _CourseFilter.all => emptyStateBuilder,
      _CourseFilter.theory =>
        (BuildContext context) =>
            _buildFilteredEmptyState(context, 'No theory marks yet'),
      _CourseFilter.lab => (BuildContext context) => _buildFilteredEmptyState(
        context,
        'No lab marks yet',
      ),
    };

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(8, 8, 8, 4),
          child: Row(
            spacing: 8,
            children: [
              Expanded(
                child: _FilterButton(
                  label: 'All (${records.length})',
                  selected: selected.value == _CourseFilter.all,
                  onPress: () => selected.value = _CourseFilter.all,
                ),
              ),
              Expanded(
                child: _FilterButton(
                  label: 'Theory (${theoryRecords.length})',
                  selected: selected.value == _CourseFilter.theory,
                  onPress: () => selected.value = _CourseFilter.theory,
                ),
              ),
              Expanded(
                child: _FilterButton(
                  label: 'Lab (${labRecords.length})',
                  selected: selected.value == _CourseFilter.lab,
                  onPress: () => selected.value = _CourseFilter.lab,
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: _MarksRecordsList(
            records: filteredRecords,
            updateTime: updateTime,
            onRefresh: onRefresh,
            refreshIndicatorColor: refreshIndicatorColor,
            emptyBuilder: emptyBuilder,
            footerBuilder: footerBuilder,
          ),
        ),
      ],
    );
  }

  static Widget _buildFilteredEmptyState(BuildContext context, String message) {
    return Center(
      child: Text(
        message,
        style: TextStyle(
          fontSize: 14,
          fontWeight: FontWeight.w500,
          color: context.theme.colors.primary,
        ),
      ),
    );
  }
}

class _FilterButton extends StatelessWidget {
  final String label;
  final bool selected;
  final VoidCallback onPress;

  const _FilterButton({
    required this.label,
    required this.selected,
    required this.onPress,
  });

  @override
  Widget build(BuildContext context) {
    return FButton(
      size: FButtonSizeVariant.sm,
      variant: selected ? FButtonVariant.primary : FButtonVariant.outline,
      selected: selected,
      onPress: onPress,
      child: Text(
        label,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
      ),
    );
  }
}

class _MarksRecordsList extends StatelessWidget {
  final List<MarksRecord> records;
  final int updateTime;
  final RefreshCallback onRefresh;
  final Color refreshIndicatorColor;
  final Widget Function(BuildContext context) emptyBuilder;
  final Widget Function(int updateTime) footerBuilder;

  const _MarksRecordsList({
    required this.records,
    required this.updateTime,
    required this.onRefresh,
    required this.refreshIndicatorColor,
    required this.emptyBuilder,
    required this.footerBuilder,
  });

  @override
  Widget build(BuildContext context) {
    return RefreshIndicator(
      onRefresh: onRefresh,
      backgroundColor: MarksColors.tableBackground,
      color: refreshIndicatorColor,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        child: ConstrainedBox(
          constraints: BoxConstraints(
            minHeight: MediaQuery.of(context).size.height * 0.72,
          ),
          child: records.isEmpty
              ? emptyBuilder(context)
              : Padding(
                  padding: const EdgeInsets.all(6),
                  child: Column(
                    spacing: 4,
                    children: [
                      for (final record in records)
                        MarksCard(
                          key: ValueKey('${record.coursecode}_${record.slot}'),
                          record: record.copyWith(marks: sortedMarks(record)),
                        ),
                      footerBuilder(updateTime),
                    ],
                  ),
                ),
        ),
      ),
    );
  }
}

List<MarksRecordEach> sortedMarks(MarksRecord record) {
  final cloned = [...record.marks];
  cloned.sort((a, b) => int.parse(a.serial).compareTo(int.parse(b.serial)));
  return cloned;
}
