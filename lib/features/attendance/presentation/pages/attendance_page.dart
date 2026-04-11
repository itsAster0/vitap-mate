import 'package:vitapmate/core/utils/extention.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/filter.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';

class AttendancePage extends HookConsumerWidget {
  const AttendancePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final filter = ref.watch(attendanceFilterProvider);
    final autoRefreshOnOpen = ref.watch(autoRefreshOnOpenProvider);

    Future<void> update() async {
      try {
        await ref.read(attendanceProvider.notifier).updateAttendance();
      } catch (e) {
        if (context.mounted) {
          try {
            disCommonToast(context, e);
          } catch (_) {}
        }
      }
    }

    useEffect(() {
      if (!autoRefreshOnOpen) {
        return null;
      }
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref.read(attendanceProvider.notifier).updateAttendance();
        } catch (_) {}
      });
      return null;
    }, [autoRefreshOnOpen]);

    final attendanceData = ref.watch(attendanceProvider);

    return RefreshIndicator(
      onRefresh: update,
      backgroundColor: context.theme.colors.primary,
      color: context.theme.colors.primaryForeground,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        child: ConstrainedBox(
          constraints: BoxConstraints(
            minHeight: MediaQuery.of(context).size.height * 0.8,
          ),
          child: attendanceData.when(
            data: (data) {
              if (data.records.isEmpty) {
                return const Center(
                  child: Padding(
                    padding: EdgeInsets.all(16),
                    child: FAlert(
                      icon: Icon(FIcons.info),
                      title: Text("No attendance yet"),
                      subtitle: Text(
                        "Pull to refresh once attendance is available.",
                      ),
                    ),
                  ),
                );
              }

              final filteredRecords = switch (filter) {
                AttendanceFilter.all => data.records,
                AttendanceFilter.classes =>
                  data.records.where((r) => !r.islab()).toList(),
                AttendanceFilter.labs =>
                  data.records.where((r) => r.islab()).toList(),
              };

              return Padding(
                padding: const EdgeInsets.all(6),
                child: Column(
                  spacing: 8,
                  children: [
                    const AttendanceFilterTabs(),
                    if (filteredRecords.isEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 40),
                        child: FAlert(
                          icon: const Icon(FIcons.info),
                          title: Text(
                            filter == AttendanceFilter.labs
                                ? "No Lab records"
                                : "No Class records",
                          ),
                          subtitle: const Text(
                            "Try another attendance filter.",
                          ),
                        ),
                      )
                    else
                      for (final i in filteredRecords.asMap().entries)
                        AttendanceCard(
                          key: ValueKey('${i.value.courseId}_${i.key}'),
                          record: i.value,
                          index: i.key,
                        ),

                    Padding(
                      padding: const EdgeInsets.only(top: 12, bottom: 20),
                      child: Text(
                        "Data updated on ${formatUnixTimestamp(data.updateTime.toInt())}",
                        style: TextStyle(fontSize: 14, color: Colors.grey[600]),
                      ),
                    ),
                  ],
                ),
              );
            },
            error: (e, _) {
              final msg = commonErrorMessage(e);
              // try {
              //   disCommonToast(context, e);
              // } catch (_) {}
              return Center(child: Text(msg));
            },
            loading:
                () => Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: _LoadingBar(
                      label: "Loading attendance...",
                      color: context.theme.colors.primary,
                    ),
                  ),
                ),
          ),
        ),
      ),
    );
  }
}

class _LoadingBar extends StatelessWidget {
  final String label;
  final Color color;

  const _LoadingBar({required this.label, required this.color});

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 280),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            style: TextStyle(
              fontSize: 14,
              color: context.theme.colors.primary,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 12),
          LinearProgressIndicator(
            minHeight: 3,
            color: color,
            backgroundColor: color.withValues(alpha: 0.18),
            borderRadius: BorderRadius.circular(8),
          ),
        ],
      ),
    );
  }
}

class AttendanceFilterTabs extends HookConsumerWidget {
  const AttendanceFilterTabs({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final filter = ref.watch(attendanceFilterProvider);
    final notifier = ref.read(attendanceFilterProvider.notifier);

    Widget tab({
      required AttendanceFilter value,
      required String label,
      required IconData icon,
    }) {
      final selected = filter == value;

      return Expanded(
        child: FButton(
          variant: selected ? FButtonVariant.primary : FButtonVariant.outline,
          size: FButtonSizeVariant.sm,
          onPress: () => notifier.setFilter(value),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 16),
              const SizedBox(width: 6),
              Flexible(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      );
    }

    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        spacing: 8,
        children: [
          tab(
            value: AttendanceFilter.all,
            label: 'All',
            icon: FIcons.listFilter,
          ),
          tab(
            value: AttendanceFilter.classes,
            label: 'Class',
            icon: FIcons.libraryBig,
          ),
          tab(
            value: AttendanceFilter.labs,
            label: 'Lab',
            icon: FIcons.flaskConical,
          ),
        ],
      ),
    );
  }
}
