import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator, RefreshCallback;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/marks_card.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksPage extends HookConsumerWidget {
  const MarksPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(marksProvider.notifier).updatemarks().catchError((e, st) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });
      return null;
    }, const []);

    Future<void> update() async {
      try {
        await ref.read(marksProvider.notifier).updatemarks();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    final marksData = ref.watch(marksProvider);

    return AnimatedSwitcher(
      duration: Motion.medium,
      child: marksData.when(
        skipLoadingOnRefresh: true,
        skipLoadingOnReload: true,
        data: (data) => _MarksView(
          key: const ValueKey('data'),
          records: data.records,
          updateTime: data.updateTime.toInt(),
          onRefresh: update,
        ),
        error: (e, _) => EmptyState(
          key: const ValueKey('error'),
          icon: FLucideIcons.cloudAlert,
          title: "Couldn't load marks",
          message: commonErrorMessage(e),
          action: FButton(
            variant: FButtonVariant.outline,
            mainAxisSize: MainAxisSize.min,
            onPress: update,
            child: const Text('Try again'),
          ),
        ),
        loading: () => const Padding(
          key: ValueKey('loading'),
          padding: EdgeInsets.all(Space.sm),
          child: Column(
            children: [
              Skeleton(height: 40, radius: Radii.md),
              SizedBox(height: Space.lg),
              SkeletonList(count: 5, height: 112),
            ],
          ),
        ),
      ),
    );
  }
}

enum _CourseFilter { all, theory, lab }

class _MarksView extends HookWidget {
  const _MarksView({
    super.key,
    required this.records,
    required this.updateTime,
    required this.onRefresh,
  });

  final List<MarksRecord> records;
  final int updateTime;
  final RefreshCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final filter = useState(_CourseFilter.all);
    final theory = records.where((r) => !r.islab()).toList();
    final labs = records.where((r) => r.islab()).toList();
    final shown = switch (filter.value) {
      _CourseFilter.all => records,
      _CourseFilter.theory => theory,
      _CourseFilter.lab => labs,
    };

    return ScreenRefresh(
      onRefresh: onRefresh,
      tasks: const ['vtop_marks'],
      child: RefreshIndicator(
        onRefresh: onRefresh,
        backgroundColor: context.theme.colors.background,
        color: context.theme.colors.foreground,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(
            Space.sm,
            Space.sm,
            Space.sm,
            Space.lg,
          ),
          physics: const AlwaysScrollableScrollPhysics(),
          children: [
            if (records.isNotEmpty) ...[
              Segmented<_CourseFilter>(
                value: filter.value,
                onChanged: (value) => filter.value = value,
                segments: [
                  (_CourseFilter.all, 'All  ${records.length}'),
                  (_CourseFilter.theory, 'Theory  ${theory.length}'),
                  (_CourseFilter.lab, 'Lab  ${labs.length}'),
                ],
              ),
              const SizedBox(height: Space.md),
            ],
            AnimatedSwitcher(
              duration: Motion.medium,
              switchInCurve: const Interval(0.3, 1, curve: Curves.easeOut),
              switchOutCurve: const Interval(0.7, 1, curve: Curves.easeIn),
              layoutBuilder: (current, previous) => Stack(
                alignment: Alignment.topCenter,
                children: [...previous, ?current],
              ),
              child: Column(
                key: ValueKey(filter.value),
                children: [
                  if (shown.isEmpty)
                    EmptyState(
                      icon: FLucideIcons.clipboardList,
                      title: records.isEmpty ? 'No marks yet' : 'Nothing here',
                      message: records.isEmpty
                          ? 'Marks appear here once faculty upload them.'
                          : 'No ${filter.value == _CourseFilter.lab ? 'lab' : 'theory'} marks yet.',
                    )
                  else
                    for (final (i, record) in shown.indexed)
                      Padding(
                        padding: const EdgeInsets.only(bottom: Space.sm + 2),
                        child: EnterFade(
                          index: i,
                          child: MarksCard(
                            key: ValueKey(
                              '${record.coursecode}_${record.slot}',
                            ),
                            record: record.copyWith(marks: sortedMarks(record)),
                          ),
                        ),
                      ),
                ],
              ),
            ),
            DataUpdatedFooter(updateTime: updateTime),
          ],
        ),
      ),
    );
  }
}

List<MarksRecordEach> sortedMarks(MarksRecord record) {
  final cloned = [...record.marks];
  cloned.sort(
    (a, b) =>
        (int.tryParse(a.serial) ?? 0).compareTo(int.tryParse(b.serial) ?? 0),
  );
  return cloned;
}
