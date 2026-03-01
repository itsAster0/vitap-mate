import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/services/google_calendar_sync_service.dart';

class CalendarSyncPage extends HookConsumerWidget {
  const CalendarSyncPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final loading = useState(false);
    final now = DateTime.now();
    final initialUntil = now.add(const Duration(days: 120));
    final calendarController = useMemoized(
      () => FCalendarController.date(
        initialSelection: DateTime.utc(
          initialUntil.year,
          initialUntil.month,
          initialUntil.day,
        ),
        toggleable: false,
      ),
    );
    useEffect(() => calendarController.dispose, [calendarController]);
    final selectedUntilUtc = useValueListenable(calendarController);
    final selectedUntilLocal = DateTime(
      (selectedUntilUtc ?? DateTime.utc(now.year, now.month, now.day)).year,
      (selectedUntilUtc ?? DateTime.utc(now.year, now.month, now.day)).month,
      (selectedUntilUtc ?? DateTime.utc(now.year, now.month, now.day)).day,
      23,
      59,
      59,
    );
    final appsLoading = useState(true);
    final googleCalendarAvailable = useState(false);
    final googleAccounts = useState<List<String>>([]);
    final selectedAccount = useState<String?>(null);
    final titleTemplateController = useTextEditingController(text: "{name}");
    final descriptionTemplateController = useTextEditingController(
      text:
          "Course: {courseCode}\nType: {courseType}\nSlot: {slot}\nFaculty: {faculty}\nSEM:{semesterId}",
    );
    final locationTemplateController = useTextEditingController(
      text: "{block}-{roomNo}",
    );

    final canSync = useMemoized(
      () =>
          Platform.isAndroid &&
          !appsLoading.value &&
          googleCalendarAvailable.value &&
          selectedAccount.value != null,
      [appsLoading.value, googleCalendarAvailable.value, selectedAccount.value],
    );

    Future<void> loadAppsAndCalendars() async {
      if (!Platform.isAndroid) {
        appsLoading.value = false;
        return;
      }
      appsLoading.value = true;
      final installed =
          await GoogleCalendarSyncService.getAvailableCalendarApps();
      final googleAppAvailable = installed.any(
        (app) => app.packageName == "com.google.android.calendar",
      );
      List<String> accounts = const [];
      try {
        accounts = await GoogleCalendarSyncService.getGoogleAccounts();
      } catch (_) {
        accounts = const [];
      }
      googleAccounts.value = accounts;
      selectedAccount.value =
          accounts.contains(selectedAccount.value)
              ? selectedAccount.value
              : accounts.firstOrNull;
      googleCalendarAvailable.value = googleAppAvailable && accounts.isNotEmpty;
      appsLoading.value = false;
    }

    useEffect(() {
      loadAppsAndCalendars();
      return null;
    }, const []);

    String formatDate(DateTime dt) {
      return "${dt.day.toString().padLeft(2, '0')}/${dt.month.toString().padLeft(2, '0')}/${dt.year}";
    }

    Future<void> showInfo(String title, String description) async {
      if (!context.mounted) return;
      showFToast(
        context: context,
        alignment: FToastAlignment.bottomCenter,
        title: Text(title),
        description: Text(description),
      );
    }

    Future<bool> ensureCalendarPermission() async {
      var status = await Permission.calendarFullAccess.status;
      if (!status.isGranted) {
        status = await Permission.calendarWriteOnly.status;
      }

      if (status.isGranted) return true;

      if (status.isPermanentlyDenied || status.isRestricted) {
        if (!context.mounted) return false;
        showFToast(
          context: context,
          alignment: FToastAlignment.bottomCenter,
          title: const Text("Permission Blocked"),
          description: const Text(
            "Calendar permission is permanently denied. Open app settings to enable it.",
          ),
          suffixBuilder:
              (context, entry) => IntrinsicHeight(
                child: FButton(
                  onPress: () async {
                    entry.dismiss();
                    await openAppSettings();
                  },
                  child: const Text("Settings"),
                ),
              ),
        );
        return false;
      }

      status = await Permission.calendarFullAccess.request();
      if (!status.isGranted) {
        status = await Permission.calendarWriteOnly.request();
      }

      if (status.isGranted) return true;

      if (status.isPermanentlyDenied || status.isRestricted) {
        if (!context.mounted) return false;
        showFToast(
          context: context,
          alignment: FToastAlignment.bottomCenter,
          title: const Text("Permission Blocked"),
          description: const Text(
            "Calendar permission is permanently denied. Open app settings to enable it.",
          ),
          suffixBuilder:
              (context, entry) => IntrinsicHeight(
                child: FButton(
                  onPress: () async {
                    entry.dismiss();
                    await openAppSettings();
                  },
                  child: const Text("Settings"),
                ),
              ),
        );
        return false;
      }

      await showInfo(
        "Permission Required",
        "Please allow Calendar permission to continue.",
      );
      return false;
    }

    Future<void> sync() async {
      if (loading.value) return;
      if (!Platform.isAndroid) {
        await showInfo(
          "Android Only",
          "Calendar sync is available only on Android.",
        );
        return;
      }

      if (!await ensureCalendarPermission()) {
        return;
      }

      var loadingDialogShown = false;
      loading.value = true;
      try {
        if (context.mounted) {
          loadingDialogShown = true;
          showAdaptiveDialog(
            context: context,
            barrierDismissible: false,
            builder:
                (_) => PopScope(
                  canPop: false,
                  child: FDialog(
                    direction: Axis.horizontal,
                    title: const Text("Syncing"),
                    body: const Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        SizedBox(
                          width: 22,
                          height: 22,
                          child: FCircularProgress.pinwheel(),
                        ),
                        SizedBox(width: 12),
                        Text("Syncing classes..."),
                      ],
                    ),
                    actions: [
                      Padding(
                        padding: EdgeInsets.symmetric(
                          horizontal: 20,
                          vertical: 2,
                        ),
                      ),
                    ],
                  ),
                ),
          );
        }

        final timetable = await ref.read(timetableProvider.future);
        final result = await GoogleCalendarSyncService.syncTimetable(
          timetable,
          selectedUntilLocal,
          accountName: selectedAccount.value,
          titleTemplate: titleTemplateController.text.trim(),
          descriptionTemplate: descriptionTemplateController.text.trim(),
          locationTemplate: locationTemplateController.text.trim(),
        );
        final count = result.created;

        if (loadingDialogShown &&
            context.mounted &&
            Navigator.of(context, rootNavigator: true).canPop()) {
          Navigator.of(context, rootNavigator: true).pop();
          loadingDialogShown = false;
        }

        if (!context.mounted) return;
        showFToast(
          context: context,
          alignment: FToastAlignment.bottomCenter,
          title:
              count > 0
                  ? const Text("Calendar Synced")
                  : const Text("No Classes Synced"),
          description: Text(
            count > 0
                ? "$count classes updated till ${formatDate(selectedUntilLocal)}${result.calendarName != null ? " in ${result.calendarName}" : ""}."
                : "No class events were created. Check selected date and timetable data.",
          ),
          suffixBuilder:
              count > 0
                  ? (context, entry) => IntrinsicHeight(
                    child: FButton(
                      onPress: () async {
                        entry.dismiss();
                        await GoogleCalendarSyncService.openCalendarApp(
                          "com.google.android.calendar",
                        );
                      },
                      child: const Text("Open"),
                    ),
                  )
                  : null,
        );
      } catch (e) {
        if (loadingDialogShown &&
            context.mounted &&
            Navigator.of(context, rootNavigator: true).canPop()) {
          Navigator.of(context, rootNavigator: true).pop();
          loadingDialogShown = false;
        }
        if (context.mounted) disCommonToast(context, e);
      } finally {
        if (loadingDialogShown &&
            context.mounted &&
            Navigator.of(context, rootNavigator: true).canPop()) {
          Navigator.of(context, rootNavigator: true).pop();
        }
        loading.value = false;
      }
    }

    return SingleChildScrollView(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          spacing: 6,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              "Calendar Sync",
              style: context.theme.typography.xl.copyWith(
                fontWeight: FontWeight.w700,
              ),
            ),

            const Text(
              "Choose until when classes should be created. Re-sync will remove older synced classes and replace them with the latest timetable.",
            ),
            FDivider(),
            Text(
              "Add Classes Till",
              style: context.theme.typography.base.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            Text(
              formatDate(selectedUntilLocal),
              style: context.theme.typography.lg,
            ),

            Center(
              child: FCalendar(
                controller: calendarController,
                start: DateTime(now.year, now.month, now.day),
                end: DateTime(now.year + 2, now.month, now.day + 1),
                initialMonth: initialUntil,
              ),
            ),
            FDivider(),

            FCard(
              title: Text(
                "Google Calendar Setup",
                style: context.theme.typography.base.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              subtitle: Text(
                "Choose which Google account should receive synced classes.",
              ),
              child:
                  appsLoading.value
                      ? const SizedBox(
                        height: 24,
                        width: 24,
                        child: FCircularProgress.pinwheel(),
                      )
                      : Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            googleCalendarAvailable.value
                                ? "Google Calendar is ready."
                                : "Google Calendar app or writable account not available.",
                          ),
                          const SizedBox(height: 8),
                          if (googleAccounts.value.isNotEmpty)
                            FSelectMenuTile<String>(
                              title: const Text("Google Account"),
                              details: Text(
                                selectedAccount.value ??
                                    googleAccounts.value.first,
                              ),
                              initialValue: selectedAccount.value,
                              menu: [
                                for (final account in googleAccounts.value)
                                  FSelectTile<String>(
                                    title: Text(account),
                                    value: account,
                                  ),
                              ],
                              onChange:
                                  loading.value
                                      ? null
                                      : (value) {
                                        final selected = value.firstOrNull;
                                        if (selected != null) {
                                          selectedAccount.value = selected;
                                        }
                                      },
                            )
                          else
                            Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                const Text("No writable Google accounts found."),
                                const SizedBox(height: 8),
                                FButton(
                                  style: FButtonStyle.outline(),
                                  onPress:
                                      (loading.value || appsLoading.value)
                                          ? null
                                          : () async {
                                            final ok =
                                                await ensureCalendarPermission();
                                            if (ok) {
                                              await loadAppsAndCalendars();
                                            }
                                          },
                                  child: const Text("Grant Permission"),
                                ),
                              ],
                            ),
                        ],
                      ),
            ),
            FCard(
              title: Text(
                "Event Layout",
                style: context.theme.typography.base.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              subtitle: const Text(
                "Customize how events look in Google Calendar.",
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  FTextField(
                    controller: titleTemplateController,
                    label: const Text("Title Template"),
                    hint: "{name}",
                  ),
                  const SizedBox(height: 8),
                  FTextField.multiline(
                    maxLines: 4,
                    controller: descriptionTemplateController,
                    label: const Text("Description Template"),
                    hint: "Course: {courseCode}",
                  ),
                  const SizedBox(height: 8),
                  FTextField(
                    controller: locationTemplateController,
                    label: const Text("Location Template"),
                    hint: "{block}-{roomNo}",
                  ),
                  const SizedBox(height: 8),
                  Text(
                    "Placeholders: {name} {courseCode} {courseType} {faculty} {slot} {day} {startTime} {endTime} {block} {roomNo} {semesterId}",
                    style: context.theme.typography.sm,
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            FButton(
              onPress: (loading.value || !canSync) ? null : sync,
              child:
                  loading.value
                      ? const SizedBox(
                        width: 22,
                        height: 22,
                        child: FCircularProgress.pinwheel(),
                      )
                      : const Text("Sync Timetable"),
            ),
            const SizedBox(height: 8),
            Text(
              "Note: Google Calendar may take a few minutes to display newly synced events.",
              style: context.theme.typography.sm,
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}
