import 'package:app_settings/app_settings.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/main.dart';

class NotificationManagementPage extends HookConsumerWidget {
  const NotificationManagementPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pauseDaysController = useTextEditingController(text: "1");
    final classReminderSettings = ref.watch(classReminderSettingsProvider);
    final examReminderSettings = ref.watch(examReminderSettingsProvider);
    final classController = useMemoized(
      () => FContinuousSliderController(
        selection: FSliderSelection(
          max: ((classReminderSettings.notifyBeforeMinutes - 5) / 55).clamp(
            0.0,
            1.0,
          ),
        ),
      ),
      [classReminderSettings.notifyBeforeMinutes],
    );
    final examController = useMemoized(
      () => FContinuousSliderController(
        selection: FSliderSelection(
          max: ((examReminderSettings.notifyBeforeMinutes - 5) / 115).clamp(
            0.0,
            1.0,
          ),
        ),
      ),
      [examReminderSettings.notifyBeforeMinutes],
    );

    return Container(
      color: context.theme.colors.background,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(8),
        child: FTileGroup(
          divider: FItemDivider.indented,
          label: const Text("Notification Management"),
          children: [
            FTile(
              prefix: Icon(FIcons.bell),
              title: const Text("System Notification Settings"),
              suffix: Icon(FIcons.chevronRight),
              onPress: () async {
                await notifications.requestPermissions();
                AppSettings.openAppSettings(type: AppSettingsType.notification);
              },
            ),
            FTile(
              prefix: Icon(FIcons.bell),
              title: const Text("Class Reminders"),
            
              suffix: FSwitch(
                value: classReminderSettings.enabled,
                onChange: (value) async {
                  if (value) {
                    final granted =
                        await notifications.requestAndroidNotificationPermission();
                    if (!granted) {
                      await ref
                          .read(classReminderSettingsControllerProvider)
                          .setEnabled(false);
                      return;
                    }
                  }

                  await ref
                      .read(classReminderSettingsControllerProvider)
                      .setEnabled(value);

                  if (value) {
                    await ref
                        .read(timetableProvider.notifier)
                        .updateTimetable();
                  }
                },
              ),
            ),
         
            FTile(
              prefix: Icon(FIcons.calendarDays),
              title: const Text("Notify Before"),
              details: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text("${classReminderSettings.notifyBeforeMinutes} minutes"),
                  FSlider(
                    controller: classController,
                    onChange: (selection) async {
                      final minutes =
                          (5 + (selection.offset.max * 55).round()).clamp(
                            5,
                            60,
                          );
                      if (minutes == classReminderSettings.notifyBeforeMinutes) {
                        return;
                      }
                      await ref
                          .read(classReminderSettingsControllerProvider)
                          .setNotifyBeforeMinutes(minutes);
                      if (ref.read(classReminderSettingsProvider).enabled) {
                        await ref.read(timetableProvider.notifier).updateTimetable();
                      }
                    },
                  ),
                ],
              ),
            ),
            FTile(
              prefix: Icon(FIcons.userCheck),
              title: const Text("Pause Class Reminders"),
          
              suffix: Icon(FIcons.chevronRight),
              onPress: () {
                showFDialog(
                  context: context,
                  builder:
                      (context, style, animation) => FDialog(
                        animation: animation,
                        direction: Axis.horizontal,
                        title: const Text("Pause class reminders"),
                        body: Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text(
                              "Enter number of days to pause class notifications.",
                            ),
                            const SizedBox(height: 8),
                            FTextField(controller: pauseDaysController),
                          ],
                        ),
                        actions: [
                          FButton(
                            style: FButtonStyle.outline(),
                            child: const Text("Clear Pause"),
                            onPress: () async {
                              await ref
                                  .read(classReminderSettingsControllerProvider)
                                  .clearPause();
                              if (context.mounted) Navigator.of(context).pop();
                              if (ref
                                  .read(classReminderSettingsProvider)
                                  .enabled) {
                                await ref
                                    .read(timetableProvider.notifier)
                                    .updateTimetable();
                              }
                            },
                          ),
                          FButton(
                            child: const Text("Pause"),
                            onPress: () async {
                              final days =
                                  int.tryParse(
                                    pauseDaysController.text.trim(),
                                  ) ??
                                  0;
                              if (days <= 0) return;
                              await ref
                                  .read(classReminderSettingsControllerProvider)
                                  .pauseForDays(days);
                              if (context.mounted) Navigator.of(context).pop();
                            },
                          ),
                        ],
                      ),
                );
              },
            ),
            FTile(
              prefix: Icon(FIcons.bell),
              title: const Text("Exam Reminders"),
         
              suffix: FSwitch(
                value: examReminderSettings.enabled,
                onChange: (value) async {
                  if (value) {
                    final granted =
                        await notifications.requestAndroidNotificationPermission();
                    if (!granted) {
                      await ref
                          .read(examReminderSettingsControllerProvider)
                          .setEnabled(false);
                      return;
                    }
                  }

                  await ref
                      .read(examReminderSettingsControllerProvider)
                      .setEnabled(value);

                  if (value) {
                    await ref
                        .read(examScheduleProvider.notifier)
                        .updatexamschedule();
                  }
                },
              ),
            ),
            FTile(
              prefix: Icon(FIcons.calendarDays),
              title: const Text("Exam Notify Before"),
              details: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text("${examReminderSettings.notifyBeforeMinutes} minutes"),
                  FSlider(
                    controller: examController,
                    onChange: (selection) async {
                      final minutes =
                          (5 + (selection.offset.max * 115).round()).clamp(
                            5,
                            120,
                          );
                      if (minutes == examReminderSettings.notifyBeforeMinutes) {
                        return;
                      }
                      await ref
                          .read(examReminderSettingsControllerProvider)
                          .setNotifyBeforeMinutes(minutes);
                      if (ref.read(examReminderSettingsProvider).enabled) {
                        await ref.read(examScheduleProvider.notifier).updatexamschedule();
                      }
                    },
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
