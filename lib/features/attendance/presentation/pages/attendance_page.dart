import 'dart:developer';

import 'package:flutter/material.dart' show RefreshIndicator, RefreshCallback;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AttendancePage extends HookConsumerWidget {
  const AttendancePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> update() async {
      try {
        await ref.read(attendanceProvider.notifier).updateAttendance();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        ref.read(attendanceProvider.notifier).updateAttendance().catchError((
          e,
          st,
        ) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });
      return null;
    }, const []);

    final attendanceData = ref.watch(attendanceProvider);

    return AnimatedSwitcher(
      duration: Motion.medium,
      child: attendanceData.when(
        skipLoadingOnRefresh: true,
        skipLoadingOnReload: true,
        data: (data) => _AttendanceView(
          key: const ValueKey('data'),
          records: data.records,
          updateTime: data.updateTime.toInt(),
          onRefresh: update,
        ),
        error: (e, _) => EmptyState(
          key: const ValueKey('error'),
          icon: FLucideIcons.cloudAlert,
          title: "Couldn't load attendance",
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
              SkeletonList(count: 5, height: 104),
            ],
          ),
        ),
      ),
    );
  }
}

enum _CourseFilter { all, theory, lab }

class _AttendanceView extends HookWidget {
  const _AttendanceView({
    super.key,
    required this.records,
    required this.updateTime,
    required this.onRefresh,
  });

  final List<AttendanceRecord> records;
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

    return RefreshIndicator(
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
                    title: records.isEmpty
                        ? 'No attendance yet'
                        : 'Nothing here',
                    message: records.isEmpty
                        ? 'Attendance shows up once classes begin.'
                        : 'No ${filter.value == _CourseFilter.lab ? 'lab' : 'theory'} courses this semester.',
                  )
                else
                  for (final (i, record) in shown.indexed)
                    Padding(
                      padding: const EdgeInsets.only(bottom: Space.sm + 2),
                      child: EnterFade(
                        index: i,
                        child: AttendanceCard(
                          key: ValueKey('${record.courseId}_$i'),
                          record: record,
                          index: i,
                        ),
                      ),
                    ),
              ],
            ),
          ),
          DataUpdatedFooter(updateTime: updateTime),
        ],
      ),
    );
  }
}
