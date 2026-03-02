import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/services/google_calendar_sync_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum _DateSelectionTab { start, end }

class CalendarSyncPage extends HookConsumerWidget {
  const CalendarSyncPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final loading = useState(false);
    final now = DateTime.now();
    final startDate = useState(DateTime(now.year, now.month, now.day));
    final endDate = useState(
      DateTime(
        now.add(const Duration(days: 30)).year,
        now.add(const Duration(days: 30)).month,
        now.add(const Duration(days: 30)).day,
        23,
        59,
        59,
      ),
    );
    final selectingDate = useState(_DateSelectionTab.end);

    final appsLoading = useState(true);
    final permissionGranted = useState(false);
    final googleCalendarAvailable = useState(false);
    final googleAccounts = useState<List<String>>([]);
    final selectedAccount = useState<String?>(null);
    final currentSemesterId = useState<String?>(null);
    final timetableChangedSinceLastSync = useState(false);
    final reminderMinutes = useState(10);

    final titleTemplateController = useTextEditingController(text: "{name}");
    final descriptionTemplateController = useTextEditingController(
      text:
          "Course: {courseCode}\nType: {courseType}\nSlot: {slot}\nFaculty: {faculty}",
    );
    final locationTemplateController = useTextEditingController(
      text: "{block}-{roomNo}",
    );

    final canSync = useMemoized(
      () =>
          Platform.isAndroid &&
          !appsLoading.value &&
          googleCalendarAvailable.value &&
          selectedAccount.value != null &&
          !endDate.value.isBefore(startDate.value),
      [
        appsLoading.value,
        googleCalendarAvailable.value,
        selectedAccount.value,
        startDate.value,
        endDate.value,
      ],
    );

    final canClear = useMemoized(
      () =>
          Platform.isAndroid &&
          !loading.value &&
          !appsLoading.value &&
          selectedAccount.value != null,
      [loading.value, appsLoading.value, selectedAccount.value],
    );

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

    String timetableHashFor(TimetableData timetable) {
      final slots =
          timetable.slots
              .where((slot) => slot.serial != "-1")
              .map(
                (slot) =>
                    "${slot.day}|${slot.slot}|${slot.courseCode}|${slot.courseType}|${slot.startTime}|${slot.endTime}|${slot.name}|${slot.faculty}|${slot.block}|${slot.roomNo}",
              )
              .toList()
            ..sort();
      final canonical =
          "${timetable.semesterId}::${timetable.updateTime}::${slots.join("||")}";

      // Stable FNV-1a 64-bit hash for local change detection.
      var hash = 0xcbf29ce484222325;
      for (final codeUnit in canonical.codeUnits) {
        hash ^= codeUnit;
        hash = (hash * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF;
      }
      return hash.toRadixString(16).padLeft(16, "0");
    }

    String endDateKey(String semesterId) =>
        "calendar_sync_end_date_$semesterId";
    String timetableHashKey(String semesterId) =>
        "calendar_sync_last_hash_$semesterId";

    Future<void> persistEndDateForCurrentSemester() async {
      final semesterId = currentSemesterId.value;
      if (semesterId == null || semesterId.isEmpty) return;
      final prefs = await SharedPreferences.getInstance();
      await prefs.setInt(
        endDateKey(semesterId),
        endDate.value.millisecondsSinceEpoch,
      );
    }

    Future<bool> ensureCalendarPermission() async {
      var status = await Permission.calendarFullAccess.status;
      if (!status.isGranted) {
        status = await Permission.calendarWriteOnly.status;
      }

      if (status.isGranted) {
        permissionGranted.value = true;
        return true;
      }

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
        permissionGranted.value = false;
        return false;
      }

      status = await Permission.calendarFullAccess.request();
      if (!status.isGranted) {
        status = await Permission.calendarWriteOnly.request();
      }

      if (status.isGranted) {
        permissionGranted.value = true;
        return true;
      }

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
        permissionGranted.value = false;
        return false;
      }

      await showInfo(
        "Permission Required",
        "Please allow Calendar permission to continue.",
      );
      permissionGranted.value = false;
      return false;
    }

    Future<void> loadAppsAndCalendars() async {
      if (!Platform.isAndroid) {
        permissionGranted.value = false;
        appsLoading.value = false;
        return;
      }

      appsLoading.value = true;
      final fullAccessGranted = await Permission.calendarFullAccess.isGranted;
      final writeOnlyGranted = await Permission.calendarWriteOnly.isGranted;
      permissionGranted.value = fullAccessGranted || writeOnlyGranted;
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

    Future<void> loadSyncState() async {
      final timetable = await ref.read(timetableProvider.future);
      final prefs = await SharedPreferences.getInstance();
      final semesterId = timetable.semesterId;
      currentSemesterId.value = semesterId;

      final cachedEndMs = prefs.getInt(endDateKey(semesterId));
      if (cachedEndMs != null) {
        final cachedEnd = DateTime.fromMillisecondsSinceEpoch(cachedEndMs);
        endDate.value = DateTime(
          cachedEnd.year,
          cachedEnd.month,
          cachedEnd.day,
          23,
          59,
          59,
        );
      }

      final oldSignature = prefs.getString(timetableHashKey(semesterId));
      final newSignature = timetableHashFor(timetable);
      timetableChangedSinceLastSync.value =
          oldSignature != null && oldSignature != newSignature;
    }

    useEffect(() {
      loadAppsAndCalendars();
      loadSyncState();
      return null;
    }, const []);

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
                    actions: const [
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
          startDate.value,
          endDate.value,
          accountName: selectedAccount.value,
          reminderMinutes: reminderMinutes.value,
          titleTemplate: titleTemplateController.text.trim(),
          descriptionTemplate: descriptionTemplateController.text.trim(),
          locationTemplate: locationTemplateController.text.trim(),
        );
        final prefs = await SharedPreferences.getInstance();
        await prefs.setString(
          timetableHashKey(timetable.semesterId),
          timetableHashFor(timetable),
        );
        await prefs.setInt(
          endDateKey(timetable.semesterId),
          endDate.value.millisecondsSinceEpoch,
        );
        timetableChangedSinceLastSync.value = false;
        currentSemesterId.value = timetable.semesterId;

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
              result.created > 0
                  ? const Text("Calendar Synced")
                  : const Text("No Classes Synced"),
          description: Text(
            result.created > 0
                ? "${result.created} classes updated from ${formatDate(startDate.value)} to ${formatDate(endDate.value)}${result.calendarName != null ? " in ${result.calendarName}" : ""}."
                : "No class events were created. Check date range and timetable data.",
          ),
          suffixBuilder:
              result.created > 0
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

    Future<void> clearSemesterEvents() async {
      if (!canClear || loading.value) return;
      if (!await ensureCalendarPermission()) return;

      final timetable = await ref.read(timetableProvider.future);
      if (!context.mounted) return;
      final confirmed = await showAdaptiveDialog<bool>(
        context: context,
        barrierDismissible: true,
        builder:
            (_) => FDialog(
              title: const Text("Clear Synced Events"),
              body: Text(
                "This removes only events synced by this app for semester ${timetable.semesterId} in the selected Google account.",
              ),
              actions: [
                FButton(
                  style: FButtonStyle.outline(),
                  onPress:
                      () =>
                          Navigator.of(context, rootNavigator: true).pop(false),
                  child: const Text("Cancel"),
                ),
                FButton(
                  style: FButtonStyle.destructive(),
                  onPress:
                      () =>
                          Navigator.of(context, rootNavigator: true).pop(true),
                  child: const Text("Clear"),
                ),
              ],
            ),
      );

      if (confirmed != true) return;

      loading.value = true;
      try {
        final (
          deleted,
          calendarName,
        ) = await GoogleCalendarSyncService.clearAllSyncedEvents(
          accountName: selectedAccount.value,
          semesterId: timetable.semesterId,
        );

        if (!context.mounted) return;
        showFToast(
          context: context,
          alignment: FToastAlignment.bottomCenter,
          title: const Text("Cleared"),
          description: Text(
            deleted > 0
                ? "$deleted synced events removed${calendarName != null ? " from $calendarName" : ""}."
                : "No synced events found for this semester.",
          ),
        );
      } catch (e) {
        if (context.mounted) disCommonToast(context, e);
      } finally {
        loading.value = false;
      }
    }

    final calendarFocusDate =
        selectingDate.value == _DateSelectionTab.start
            ? startDate.value
            : endDate.value;

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
              "Pick start and end dates. Re-sync clears previously synced events for the same semester, then adds fresh classes.",
            ),
            const FDivider(),

            Text(
              "Date Range",
              style: context.theme.typography.base.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            Row(
              children: [
                Expanded(
                  child: FButton(
                    style:
                        selectingDate.value == _DateSelectionTab.start
                            ? FButtonStyle.primary()
                            : FButtonStyle.outline(),
                    onPress:
                        loading.value
                            ? null
                            : () {
                              selectingDate.value = _DateSelectionTab.start;
                            },
                    child: Text("Start: ${formatDate(startDate.value)}"),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: FButton(
                    style:
                        selectingDate.value == _DateSelectionTab.end
                            ? FButtonStyle.primary()
                            : FButtonStyle.outline(),
                    onPress:
                        loading.value
                            ? null
                            : () {
                              selectingDate.value = _DateSelectionTab.end;
                            },
                    child: Text("End: ${formatDate(endDate.value)}"),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Center(
              child: FCalendar(
                key: ValueKey(
                  "${selectingDate.value.name}-${calendarFocusDate.year}-${calendarFocusDate.month}",
                ),
                controller: FCalendarController.date(
                  initialSelection: calendarFocusDate,
                ),
                onPress: (selected) {
                  final pickedStart = DateTime(
                    selected.year,
                    selected.month,
                    selected.day,
                  );
                  final pickedEnd = DateTime(
                    selected.year,
                    selected.month,
                    selected.day,
                    23,
                    59,
                    59,
                  );

                  if (selectingDate.value == _DateSelectionTab.start) {
                    startDate.value = pickedStart;
                    if (endDate.value.isBefore(startDate.value)) {
                      endDate.value = pickedEnd;
                      Future.microtask(persistEndDateForCurrentSemester);
                    }
                  } else {
                    endDate.value = pickedEnd;
                    Future.microtask(persistEndDateForCurrentSemester);
                    if (endDate.value.isBefore(startDate.value)) {
                      startDate.value = pickedStart;
                    }
                  }
                },
                start: DateTime(now.year - 1, 1, 1),
                end: DateTime(now.year + 3, 1, 1),
                initialMonth: calendarFocusDate,
              ),
            ),
            if (endDate.value.isBefore(startDate.value))
              Text(
                "End date must be on or after start date.",
                style: context.theme.typography.sm,
              ),

            const FDivider(),
            if (timetableChangedSinceLastSync.value)
              FCard(
                title: Text(
                  "Timetable Changed",
                  style: context.theme.typography.base.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                subtitle: const Text(
                  "New timetable changes were detected for this semester.",
                ),
                child: const Text(
                  "Please sync again to update your calendar with latest classes.",
                ),
              ),

            FCard(
              title: Text(
                "Google Calendar Setup",
                style: context.theme.typography.base.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              subtitle: const Text(
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
                            const Text("No writable Google accounts found."),
                          const SizedBox(height: 8),
                          Wrap(
                            spacing: 8,
                            runSpacing: 8,
                            children: [
                              if (!permissionGranted.value)
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
                              FButton(
                                style: FButtonStyle.outline(),
                                onPress:
                                    (loading.value || appsLoading.value)
                                        ? null
                                        : loadAppsAndCalendars,
                                child: const Text("Reload Accounts"),
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
                  FSelectMenuTile<int>(
                    title: const Text("Reminder"),
                    details: Text(
                      reminderMinutes.value == 0
                          ? "Off"
                          : "${reminderMinutes.value} min before",
                    ),
                    initialValue: reminderMinutes.value,
                    menu: const [
                      FSelectTile<int>(title: Text("Off"), value: 0),
                      FSelectTile<int>(title: Text("5 min before"), value: 5),
                      FSelectTile<int>(title: Text("10 min before"), value: 10),
                      FSelectTile<int>(title: Text("15 min before"), value: 15),
                      FSelectTile<int>(title: Text("30 min before"), value: 30),
                      FSelectTile<int>(title: Text("60 min before"), value: 60),
                    ],
                    onChange:
                        loading.value
                            ? null
                            : (value) {
                              final selected = value.firstOrNull;
                              if (selected != null) {
                                reminderMinutes.value = selected;
                              }
                            },
                  ),
                  const SizedBox(height: 8),
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
                    "Placeholders: {name} {courseCode} {courseType} {faculty} {slot} {day} {startTime} {endTime} {block} {roomNo}",
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
            FAccordion(
              children: [
                FAccordionItem(
                  title: const Text("Danger Zone: Clear Synced Events"),
                  initiallyExpanded: false,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Text(
                        "Clear only removes events tagged by this app for the current semester, not your other calendar events.",
                        style: context.theme.typography.sm,
                        textAlign: TextAlign.center,
                      ),
                      const SizedBox(height: 8),
                      FButton(
                        style: FButtonStyle.destructive(),
                        onPress: canClear ? clearSemesterEvents : null,
                        child: const Text(
                          "Clear Synced Events (This Semester)",
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            const SizedBox(height: 4),
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
