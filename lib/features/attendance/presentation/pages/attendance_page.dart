import 'dart:developer';
import 'package:vitapmate/core/utils/extention.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/filter.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';

class AttendancePage extends HookConsumerWidget {
  const AttendancePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final filter = ref.watch(attendanceFilterNotifierProvider);

    Future<void> update() async {
      try {
        await ref.read(attendanceProvider.notifier).updateAttendance();
      } catch (e) {
        log("$e");
        if (context.mounted) {
          try {
            disCommonToast(context, e);
          } catch (_) {}
        }
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref.read(attendanceProvider.notifier).updateAttendance();
        } catch (_) {}
      });
      return null;
    }, []);

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
                    padding: EdgeInsets.only(top: 40),
                    child: Text("No Data to show yet"),
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
                  spacing: 4,
                  children: [
                    if (filteredRecords.isEmpty)
                      const Padding(
                        padding: EdgeInsets.only(top: 40),
                        child: Text("No records for this filter"),
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
              try {
                disCommonToast(context, e);
              } catch (_) {}
              return Center(child: Text(msg));
            },
            loading:
                () => Center(
                  child: SizedBox(
                    width: 50,
                    height: 50,
                    child: CircularProgressIndicator(
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

class AttendanceHeader extends HookConsumerWidget {
  const AttendanceHeader({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final filter = ref.watch(attendanceFilterNotifierProvider);
    final notifier = ref.read(attendanceFilterNotifierProvider.notifier);

    Widget tick(AttendanceFilter value) {
      return filter == value
          ? const Icon(FIcons.check, size: 16)
          : const SizedBox(width: 16);
    }

    return FPopoverMenu(
      menuAnchor: Alignment.topRight,
      childAnchor: Alignment.bottomRight,
      menu: [
        FItemGroup(
          children: [
            FItem(
              title: const Text('All'),
              prefix: tick(AttendanceFilter.all),
              onPress: () {
                notifier.setFilter(AttendanceFilter.all);
              },
            ),
            FItem(
              prefix: tick(AttendanceFilter.classes),
              title: const Text('Classes'),

              onPress: () {
                notifier.setFilter(AttendanceFilter.classes);
              },
            ),
            FItem(
              prefix: tick(AttendanceFilter.labs),
              title: const Text('Labs'),

              onPress: () {
                notifier.setFilter(AttendanceFilter.labs);
              },
            ),
          ],
        ),
      ],
      builder:
          (_, controller, _) => FHeaderAction(
            icon: const Icon(FIcons.ellipsis),
            onPress: controller.toggle,
          ),
    );
  }
}
