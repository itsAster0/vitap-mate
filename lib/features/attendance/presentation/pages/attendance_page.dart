import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';

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

              final filteredRecords = data.records.toList();

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
              // try {
              //   disCommonToast(context, e);
              // } catch (_) {}
              return Center(child: Text(msg));
            },
            loading: () => Center(
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
