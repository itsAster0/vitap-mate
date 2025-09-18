import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/widgets/attendance.dart';

class AttendancePage extends HookConsumerWidget {
  const AttendancePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> update() async {
      try {
        await ref.read(attendanceProvider.notifier).updateAttendance();
      } catch (e) {
        log("$e");
        try {
          if (context.mounted) disCommonToast(context, e);
        } catch (e) {
          ();
        }
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref.read(attendanceProvider.notifier).updateAttendance();
        } catch (e, _) {
          ();
        }
      });
      return null;
    }, []);

    var attendanceData = ref.watch(attendanceProvider);
    return RefreshIndicator(
      onRefresh: () async {
        await update();
      },
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
                return Center(child: Text("No Data to show yet"));
              }
              return Padding(
                padding: const EdgeInsets.all(6),
                child: Column(
                  spacing: 4,
                  children: [
                    for (final i in data.records.asMap().entries)
                      AttendanceCard(
                        key: ValueKey('${i.value.courseId}_${i.key}'),
                        record: i.value,
                        index: i.key,
                      ),
                    Center(
                      child: Padding(
                        padding: const EdgeInsets.only(top: 12, bottom: 20),
                        child: Text(
                          "Data updated on ${formatUnixTimestamp(data.updateTime.toInt())}",
                          style: TextStyle(
                            fontSize: 14,
                            color: Colors.grey[600],
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
              try {
                disCommonToast(context, e);
              } catch (e) {
                ();
              }
              return Center(child: Text(msg));
            },
            loading: () {
              return Center(
                child: SizedBox(
                  width: 50,
                  height: 50,
                  child: CircularProgressIndicator(color: context.theme.colors.primaryForeground),
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}
