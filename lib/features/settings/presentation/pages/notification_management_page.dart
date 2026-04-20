import 'dart:developer';

import 'package:app_settings/app_settings.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';

double _classReminderSliderValue(int minutes) =>
    ((minutes - 5) / 55).clamp(0.0, 1.0);

double _examReminderSliderValue(int minutes) =>
    ((minutes - 5) / 115).clamp(0.0, 1.0);

int _classReminderMinutes(FSliderValue selection) =>
    (5 + (selection.max * 55).round()).clamp(5, 60).toInt();

int _examReminderMinutes(FSliderValue selection) =>
    (5 + (selection.max * 115).round()).clamp(5, 120).toInt();

class _AnimatedMinutesLabel extends StatelessWidget {
  const _AnimatedMinutesLabel(this.minutes);

  final int minutes;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 24,
      child: AnimatedSwitcher(
        duration: const Duration(milliseconds: 180),
        switchInCurve: Curves.easeOutCubic,
        switchOutCurve: Curves.easeInCubic,
        transitionBuilder: (child, animation) {
          final offset = Tween<Offset>(
            begin: const Offset(0, 0.35),
            end: Offset.zero,
          ).animate(animation);

          return FadeTransition(
            opacity: animation,
            child: SlideTransition(position: offset, child: child),
          );
        },
        child: Text("$minutes minutes", key: ValueKey(minutes)),
      ),
    );
  }
}

class NotificationManagementPage extends HookConsumerWidget {
  const NotificationManagementPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pauseDaysController = useTextEditingController(text: "1");
    final debugDelayController = useTextEditingController(text: "0");
    final classReminderSettings = ref.watch(classReminderSettingsProvider);
    final examReminderSettings = ref.watch(examReminderSettingsProvider);
    final classNotifyMinutes = useState(
      classReminderSettings.notifyBeforeMinutes,
    );
    final examNotifyMinutes = useState(
      examReminderSettings.notifyBeforeMinutes,
    );
    useEffect(() {
      classNotifyMinutes.value = classReminderSettings.notifyBeforeMinutes;
      return null;
    }, [classReminderSettings.notifyBeforeMinutes]);
    useEffect(() {
      examNotifyMinutes.value = examReminderSettings.notifyBeforeMinutes;
      return null;
    }, [examReminderSettings.notifyBeforeMinutes]);

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
                await Permission.notification.request();
                AppSettings.openAppSettings(type: AppSettingsType.notification);
              },
            ),
            if (kDebugMode)
              FTile(
                prefix: Icon(FIcons.bug),
                title: const Text("Test Notification (Debug)"),
                suffix: const Text("Send"),
                onPress: () async {
                  await showFDialog(
                    context: context,
                    builder: (context, style, animation) => FDialog(
                      animation: animation,
                      direction: Axis.horizontal,
                      title: const Text("Debug Notification Delay"),
                      body: Column(
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          const Text("Delay in seconds (0 = immediate)."),
                          const SizedBox(height: 8),
                          FTextField(
                            control: FTextFieldControl.managed(
                              controller: debugDelayController,
                            ),
                          ),
                        ],
                      ),
                      actions: [
                        FButton(
                          variant: FButtonVariant.outline,
                          onPress: () => Navigator.of(context).pop(),
                          child: const Text("Cancel"),
                        ),
                        FButton(
                          onPress: () async {
                            final status = await Permission.notification
                                .request();
                            final granted = status.isGranted;
                            if (!granted) return;
                            final delay =
                                int.tryParse(
                                  debugDelayController.text.trim(),
                                ) ??
                                0;
                            if (context.mounted) {
                              Navigator.of(context).pop();
                            }
                          },
                          child: const Text("Send"),
                        ),
                      ],
                    ),
                  );
                },
              ),
            FTile(
              prefix: Icon(FIcons.bell),
              title: const Text("Class Reminders"),

              suffix: FSwitch(
                value: classReminderSettings.enabled,
                onChange: (value) async {
                  if (value) {
                    final granted = await Permission.notification
                        .request()
                        .isGranted;
                    if (!context.mounted) return;
                    if (!granted) {
                      await setClassReminderEnabled(ref, false);
                      return;
                    }
                  }

                  await setClassReminderEnabled(ref, value);
                  if (!context.mounted) return;

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
                  _AnimatedMinutesLabel(classNotifyMinutes.value),
                  FSlider(
                    control: FSliderControl.liftedContinuous(
                      value: FSliderValue(
                        max: _classReminderSliderValue(
                          classNotifyMinutes.value,
                        ),
                      ),
                      onChange: (selection) async {
                        try {
                          final minutes = _classReminderMinutes(selection);
                          classNotifyMinutes.value = minutes;
                          if (minutes ==
                              classReminderSettings.notifyBeforeMinutes) {
                            return;
                          }
                          await setClassReminderNotifyBeforeMinutes(
                            ref,
                            minutes,
                          );
                          if (!context.mounted) return;
                          if (ref.read(classReminderSettingsProvider).enabled) {
                            await ref
                                .read(timetableProvider.notifier)
                                .updateTimetable();
                          }
                        } catch (e, st) {
                          log(
                            'Error updating class reminder notify before: $e',
                            stackTrace: st,
                          );
                        }
                      },
                    ),
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
                  builder: (context, style, animation) => FDialog(
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
                        FTextField(
                          control: FTextFieldControl.managed(
                            controller: pauseDaysController,
                          ),
                        ),
                      ],
                    ),
                    actions: [
                      FButton(
                        variant: FButtonVariant.outline,
                        child: const Text("Clear Pause"),
                        onPress: () async {
                          await clearClassReminderPause(ref);
                          if (!context.mounted) return;
                          if (context.mounted) Navigator.of(context).pop();
                          if (ref.read(classReminderSettingsProvider).enabled) {
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
                              int.tryParse(pauseDaysController.text.trim()) ??
                              0;
                          if (days <= 0) return;
                          await pauseClassRemindersForDays(ref, days);
                          if (!context.mounted) return;
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
                    final granted = await Permission.notification
                        .request()
                        .isGranted;
                    if (!context.mounted) return;
                    if (!granted) {
                      await setExamReminderEnabled(ref, false);
                      return;
                    }
                  }

                  await setExamReminderEnabled(ref, value);
                  if (!context.mounted) return;

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
                  _AnimatedMinutesLabel(examNotifyMinutes.value),
                  FSlider(
                    control: FSliderControl.liftedContinuous(
                      value: FSliderValue(
                        max: _examReminderSliderValue(examNotifyMinutes.value),
                      ),
                      onChange: (selection) async {
                        final minutes = _examReminderMinutes(selection);
                        examNotifyMinutes.value = minutes;
                        if (minutes ==
                            examReminderSettings.notifyBeforeMinutes) {
                          return;
                        }
                        await setExamReminderNotifyBeforeMinutes(ref, minutes);
                        if (!context.mounted) return;
                        if (ref.read(examReminderSettingsProvider).enabled) {
                          await ref
                              .read(examScheduleProvider.notifier)
                              .updatexamschedule();
                        }
                      },
                    ),
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
