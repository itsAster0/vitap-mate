import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/widgets/data_updated_footer.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AttendancePage extends HookConsumerWidget {
  const AttendancePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final autoRefresh = ref.watch(autoRefreshProvider);
    Future<void> update() async {
      try {
        await ref.read(attendanceProvider.notifier).updateAttendance();
      } catch (e) {
        log("$e");
      }
    }

    useEffect(() {
      if (!autoRefresh) return null;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        ref.read(attendanceProvider.notifier).updateAttendance().catchError((
          e,
          st,
        ) {
          log('auto refresh failed: $e', stackTrace: st);
        });
      });
      return null;
    }, [autoRefresh]);

    final attendanceData = ref.watch(attendanceProvider);

    return attendanceData.when(
      data: (data) {
        if (data.records.isEmpty) {
          return _AttendanceFilterView(
            records: const [],
            updateTime: data.updateTime.toInt(),
            onRefresh: update,
          );
        }

        return _AttendanceFilterView(
          records: data.records,
          updateTime: data.updateTime.toInt(),
          onRefresh: update,
        );
      },
      error: (e, _) {
        final msg = commonErrorMessage(e);
        // try {
        //   disCommonToast(context, e);
        // } catch (_) {}
        return Center(child: Text(msg));
      },
      loading: () => Center(
        child: SizedBox(
          width: 50,
          height: 50,
          child: CircularProgressIndicator(color: context.theme.colors.primary),
        ),
      ),
    );
  }
}

enum _CourseFilter { all, theory, lab }

class _AttendanceFilterView extends HookWidget {
  final List<AttendanceRecord> records;
  final int updateTime;
  final RefreshCallback onRefresh;

  const _AttendanceFilterView({
    required this.records,
    required this.updateTime,
    required this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    final selected = useState(_CourseFilter.all);
    final filteredRecords = switch (selected.value) {
      _CourseFilter.all => records,
      _CourseFilter.theory =>
        records.where((record) => !record.islab()).toList(),
      _CourseFilter.lab => records.where((record) => record.islab()).toList(),
    };
    final emptyMessage = switch (selected.value) {
      _CourseFilter.all => 'No Data to show yet',
      _CourseFilter.theory => 'No theory records for this semester',
      _CourseFilter.lab => 'No lab records for this semester',
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
                  label:
                      'Theory (${records.where((record) => !record.islab()).length})',
                  selected: selected.value == _CourseFilter.theory,
                  onPress: () => selected.value = _CourseFilter.theory,
                ),
              ),
              Expanded(
                child: _FilterButton(
                  label:
                      'Lab (${records.where((record) => record.islab()).length})',
                  selected: selected.value == _CourseFilter.lab,
                  onPress: () => selected.value = _CourseFilter.lab,
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: _AttendanceRecordsList(
            records: filteredRecords,
            updateTime: updateTime,
            onRefresh: onRefresh,
            emptyMessage: emptyMessage,
          ),
        ),
      ],
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

class _AttendanceRecordsList extends StatelessWidget {
  final List<AttendanceRecord> records;
  final int updateTime;
  final RefreshCallback onRefresh;
  final String emptyMessage;

  const _AttendanceRecordsList({
    required this.records,
    required this.updateTime,
    required this.onRefresh,
    required this.emptyMessage,
  });

  @override
  Widget build(BuildContext context) {
    return RefreshIndicator(
      onRefresh: onRefresh,
      backgroundColor: context.theme.colors.primary,
      color: context.theme.colors.primaryForeground,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(8),
        physics: const AlwaysScrollableScrollPhysics(),
        child: Column(
          spacing: 4,
          children: [
            if (records.isEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 40),
                child: Text(emptyMessage),
              )
            else
              for (final i in records.asMap().entries)
                AttendanceCard(
                  key: ValueKey('${i.value.courseId}_${i.key}'),
                  record: i.value,
                  index: i.key,
                ),
            DataUpdatedFooter(updateTime: updateTime, fontSize: 14),
          ],
        ),
      ),
    );
  }
}
