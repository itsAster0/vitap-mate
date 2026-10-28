import 'dart:io';

import 'package:intl/intl.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/ui/fit_calendar.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:vitapmate/ui/app_card.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/features/timetable/google_calendar_sync_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum _DateSelectionTab { start, end }

DateTime _startOfDay(DateTime d) => DateTime(d.year, d.month, d.day);
DateTime _endOfDay(DateTime d) => DateTime(d.year, d.month, d.day, 23, 59, 59);

String _endDateKey(String semesterId) => "calendar_sync_end_date_$semesterId";
String _timetableHashKey(String semesterId) =>
    "calendar_sync_last_hash_$semesterId";

/// FNV-1a of the synced fields, to notice a timetable change since the last
/// sync.
String _timetableHash(TimetableData timetable) {
  final slots = [
    for (final s in timetable.slots)
      if (s.serial != "-1")
        "${s.day}|${s.slot}|${s.courseCode}|${s.courseType}|${s.startTime}|${s.endTime}|${s.name}|${s.faculty}|${s.block}|${s.roomNo}",
  ]..sort();
  final canonical =
      "${timetable.semesterId}::${timetable.updateTime}::${slots.join("||")}";
  var hash = 0xcbf29ce484222325;
  for (final codeUnit in canonical.codeUnits) {
    hash ^= codeUnit;
    hash = (hash * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF;
  }
  return hash.toRadixString(16).padLeft(16, "0");
}

class CalendarSyncPage extends HookConsumerWidget {
  const CalendarSyncPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final loading = useState(false);
    final now = DateTime.now();
    final startDate = useState(_startOfDay(now));
    final endDate = useState(_endOfDay(now.add(const Duration(days: 30))));
    final selectingDate = useState(_DateSelectionTab.end);

    final calendarsLoading = useState(true);
    final permissionGranted = useState(false);
    final writableCalendars = useState<List<CalendarInfo>>([]);
    final selectedCalendarId = useState<String?>(null);
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

    final writableCalendarAvailable = writableCalendars.value.isNotEmpty;
    final canSync =
        Platform.isAndroid &&
        !calendarsLoading.value &&
        writableCalendarAvailable &&
        selectedCalendarId.value != null &&
        !endDate.value.isBefore(startDate.value);
    final canClear =
        Platform.isAndroid &&
        !loading.value &&
        !calendarsLoading.value &&
        selectedCalendarId.value != null;

    String formatDate(DateTime dt) =>
        DateFormat('dd/MM/yyyy', 'en_US').format(dt);

    // Short enough for half-width buttons on narrow phones: "8 Oct", with
    // the year only when it is not this one.
    String buttonDate(DateTime dt) => DateFormat(
      dt.year == DateTime.now().year ? 'd MMM' : 'd MMM yy',
      'en_US',
    ).format(dt);

    void toast(Widget title, String description) {
      if (!context.mounted) return;
      showFToast(
        context: context,
        alignment: .bottomCenter,
        title: title,
        description: Text(description),
      );
    }

    Future<void> persistEndDateForCurrentSemester() async {
      final semesterId = currentSemesterId.value;
      if (semesterId == null || semesterId.isEmpty) return;
      final prefs = await SharedPreferences.getInstance();
      await prefs.setInt(
        _endDateKey(semesterId),
        endDate.value.millisecondsSinceEpoch,
      );
    }

    Future<bool> ensureCalendarPermission() async {
      var status = await Permission.calendarFullAccess.status;
      if (!status.isGranted) status = await Permission.calendarWriteOnly.status;
      if (!status.isGranted &&
          (status.isPermanentlyDenied || status.isRestricted)) {
        if (!context.mounted) return false;
        showFToast(
          context: context,
          alignment: .bottomCenter,
          title: const Text("Permission Blocked"),
          description: const Text(
            "Calendar permission is permanently denied. Open app settings to enable it.",
          ),
          suffixBuilder: (context, entry) => IntrinsicHeight(
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
      if (!status.isGranted) {
        status = await Permission.calendarFullAccess.request();
        if (!status.isGranted) {
          status = await Permission.calendarWriteOnly.request();
        }
      }
      permissionGranted.value = status.isGranted;
      if (!status.isGranted) {
        toast(
          const Text("Permission Required"),
          "Please allow Calendar permission to continue.",
        );
      }
      return status.isGranted;
    }

    Future<void> loadCalendars() async {
      if (!Platform.isAndroid) {
        permissionGranted.value = false;
        calendarsLoading.value = false;
        return;
      }
      calendarsLoading.value = true;
      permissionGranted.value =
          await Permission.calendarFullAccess.isGranted ||
          await Permission.calendarWriteOnly.isGranted;
      final calendars = await CalendarSyncService.getWritableCalendars()
          .catchError((_) => <CalendarInfo>[]);
      writableCalendars.value = calendars;
      if (!calendars.any((item) => item.id == selectedCalendarId.value)) {
        selectedCalendarId.value = calendars.firstOrNull?.id;
      }
      calendarsLoading.value = false;
    }

    Future<void> loadSyncState() async {
      final timetable = await ref.read(timetableProvider.future);
      final prefs = await SharedPreferences.getInstance();
      final semesterId = timetable.semesterId;
      currentSemesterId.value = semesterId;
      if (prefs.getInt(_endDateKey(semesterId)) case final ms?) {
        endDate.value = _endOfDay(DateTime.fromMillisecondsSinceEpoch(ms));
      }
      final oldSignature = prefs.getString(_timetableHashKey(semesterId));
      timetableChangedSinceLastSync.value =
          oldSignature != null && oldSignature != _timetableHash(timetable);
    }

    useEffect(() {
      loadCalendars();
      loadSyncState();
      return null;
    }, const []);

    Future<void> sync() async {
      if (loading.value) return;
      if (!Platform.isAndroid) {
        toast(
          const Text("Android Only"),
          "Calendar sync is available only on Android.",
        );
        return;
      }
      if (!await ensureCalendarPermission()) return;

      var loadingDialogShown = false;
      void closeDialog() {
        if (loadingDialogShown &&
            context.mounted &&
            Navigator.of(context, rootNavigator: true).canPop()) {
          Navigator.of(context, rootNavigator: true).pop();
          loadingDialogShown = false;
        }
      }

      loading.value = true;
      try {
        if (context.mounted) {
          loadingDialogShown = true;
          showAdaptiveDialog(
            context: context,
            barrierDismissible: false,
            builder: (_) => const PopScope(
              canPop: false,
              child: AppDialog(
                direction: .horizontal,
                title: Text("Syncing"),
                body: Row(
                  mainAxisSize: .min,
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
                  Padding(padding: .symmetric(horizontal: 20, vertical: 2)),
                ],
              ),
            ),
          );
        }

        final timetable = await ref.read(timetableProvider.future);
        final selectedCalendar = selectedCalendarId.value;
        if (selectedCalendar == null || selectedCalendar.isEmpty) {
          throw Exception("No writable calendar selected");
        }

        final result = await CalendarSyncService.syncTimetable(
          timetable,
          startDate.value,
          endDate.value,
          calendarId: selectedCalendar,
          reminderMinutes: reminderMinutes.value,
          titleTemplate: titleTemplateController.text.trim(),
          descriptionTemplate: descriptionTemplateController.text.trim(),
          locationTemplate: locationTemplateController.text.trim(),
        );

        final prefs = await SharedPreferences.getInstance();
        await prefs.setString(
          _timetableHashKey(timetable.semesterId),
          _timetableHash(timetable),
        );
        await prefs.setInt(
          _endDateKey(timetable.semesterId),
          endDate.value.millisecondsSinceEpoch,
        );
        timetableChangedSinceLastSync.value = false;
        currentSemesterId.value = timetable.semesterId;

        closeDialog();
        toast(
          result.created > 0
              ? const Text("Calendar Synced")
              : const Text("No Classes Synced"),
          result.created > 0
              ? "${result.created} recurring classes synced as v${result.version} from ${formatDate(startDate.value)} to ${formatDate(endDate.value)}${result.calendarName != null ? " in ${result.calendarName}" : ""}."
              : "No class events were created. Check date range and timetable data.",
        );
      } catch (e) {
        closeDialog();
        if (context.mounted) disCommonToast(context, e);
      } finally {
        closeDialog();
        loading.value = false;
      }
    }

    Future<void> clearSemesterEvents() async {
      if (!canClear || loading.value) return;
      if (!await ensureCalendarPermission()) return;

      final timetable = await ref.read(timetableProvider.future);
      if (!context.mounted) return;
      final confirmed = await confirm(
        context,
        title: 'Clear Synced Events',
        body:
            'This removes only events synced by this app for semester ${timetable.semesterId} in the selected calendar.',
        action: 'Clear',
        destructive: true,
      );
      if (!confirmed) return;

      final selectedCalendar = selectedCalendarId.value;
      if (selectedCalendar == null || selectedCalendar.isEmpty) {
        if (context.mounted) {
          disCommonToast(context, Exception("No writable calendar selected"));
        }
        return;
      }

      loading.value = true;
      try {
        final (
          deleted,
          calendarName,
        ) = await CalendarSyncService.clearAllSyncedEvents(
          calendarId: selectedCalendar,
          semesterId: timetable.semesterId,
        );
        toast(
          const Text("Cleared"),
          deleted > 0
              ? "$deleted synced events removed${calendarName != null ? " from $calendarName" : ""}."
              : "No synced events found for this semester.",
        );
      } catch (e) {
        if (context.mounted) disCommonToast(context, e);
      } finally {
        loading.value = false;
      }
    }

    final calendarFocusDate = selectingDate.value == .start
        ? startDate.value
        : endDate.value;
    final selectedCalendar = writableCalendars.value
        .where((item) => item.id == selectedCalendarId.value)
        .firstOrNull;
    final typography = context.theme.typography;
    final heading = typography.body.md.copyWith(fontWeight: .w600);
    final busy = loading.value || calendarsLoading.value;

    Widget dateButton(_DateSelectionTab tab, String label, DateTime date) =>
        Expanded(
          child: FButton(
            variant: selectingDate.value == tab ? .primary : .outline,
            onPress: loading.value ? null : () => selectingDate.value = tab,
            child: Text("$label · ${buttonDate(date)}"),
          ),
        );

    return SingleChildScrollView(
      child: Padding(
        padding: const .all(16),
        child: Column(
          spacing: 6,
          crossAxisAlignment: .stretch,
          children: [
            Text(
              "Calendar Sync",
              style: typography.body.xl.copyWith(fontWeight: .w700),
            ),
            const Text(
              "Pick start and end dates, then sync your timetable to calendar.",
            ),
            const FDivider(),
            Text("Date Range", style: heading),
            Row(
              children: [
                dateButton(.start, "Start", startDate.value),
                const SizedBox(width: 8),
                dateButton(.end, "End", endDate.value),
              ],
            ),
            const SizedBox(height: 8),
            Center(
              child: FitCalendar(
                boxed: true,
                key: ValueKey(
                  "${selectingDate.value.name}-${calendarFocusDate.year}-${calendarFocusDate.month}",
                ),
                control: FGridCalendarControl(
                  start: DateTime(now.year - 1, 1, 1),
                  end: DateTime(now.year + 3, 1, 1),
                  initial: calendarFocusDate,
                ),
                selectionControl: FDateSelectionControl.liftedSingle(
                  value: calendarFocusDate,
                  onChange: (_) {},
                ),
                onDayPress: (selected) {
                  if (selectingDate.value == .start) {
                    startDate.value = _startOfDay(selected);
                    if (endDate.value.isBefore(startDate.value)) {
                      endDate.value = _endOfDay(selected);
                      Future.microtask(persistEndDateForCurrentSemester);
                    }
                  } else {
                    endDate.value = _endOfDay(selected);
                    Future.microtask(persistEndDateForCurrentSemester);
                    if (endDate.value.isBefore(startDate.value)) {
                      startDate.value = _startOfDay(selected);
                    }
                  }
                },
              ),
            ),
            if (endDate.value.isBefore(startDate.value))
              Text(
                "End date must be on or after start date.",
                style: typography.body.sm,
              ),
            const FDivider(),
            if (timetableChangedSinceLastSync.value)
              AppCard(
                title: Text("Timetable Changed", style: heading),
                subtitle: const Text(
                  "New timetable changes were detected for this semester.",
                ),
                child: const Text(
                  "Please sync again to update your calendar with latest classes.",
                ),
              ),
            AppCard(
              title: Text("Calendar Setup", style: heading),
              subtitle: const Text(
                "Choose writable calendar for recurring sync.",
              ),
              child: calendarsLoading.value
                  ? const SizedBox(
                      height: 24,
                      width: 24,
                      child: FCircularProgress.pinwheel(),
                    )
                  : Column(
                      crossAxisAlignment: .start,
                      children: [
                        Text(
                          writableCalendarAvailable
                              ? "Writable calendar is ready."
                              : "No writable calendar available.",
                        ),
                        const SizedBox(height: 8),
                        if (writableCalendarAvailable)
                          FSelectMenuTile<String>(
                            title: const Text("Calendar"),
                            details: Text(
                              selectedCalendar?.name ??
                                  writableCalendars.value.first.name,
                            ),
                            selectControl: .managedRadio(
                              initial: selectedCalendarId.value,
                              onChange: loading.value
                                  ? null
                                  : (value) {
                                      if (value.firstOrNull case final id?) {
                                        selectedCalendarId.value = id;
                                      }
                                    },
                            ),
                            menu: [
                              for (final calendar in writableCalendars.value)
                                FSelectTile<String>(
                                  title: Text(
                                    calendar.accountName.isEmpty
                                        ? calendar.name
                                        : "${calendar.name} (${calendar.accountName})",
                                  ),
                                  value: calendar.id,
                                ),
                            ],
                          )
                        else
                          const Text("No writable calendars found."),
                        const SizedBox(height: 8),
                        Wrap(
                          spacing: 8,
                          runSpacing: 8,
                          children: [
                            if (!permissionGranted.value)
                              FButton(
                                variant: .outline,
                                onPress: busy
                                    ? null
                                    : () async {
                                        if (await ensureCalendarPermission()) {
                                          await loadCalendars();
                                        }
                                      },
                                child: const Text("Grant Permission"),
                              ),
                            FButton(
                              variant: .outline,
                              onPress: busy ? null : loadCalendars,
                              child: const Text("Reload Calendars"),
                            ),
                          ],
                        ),
                      ],
                    ),
            ),
            AppCard(
              title: Text("Event Layout", style: heading),
              subtitle: const Text(
                "Reminder and how events read in your calendar.",
              ),
              child: Column(
                crossAxisAlignment: .stretch,
                children: [
                  FSelectMenuTile<int>(
                    title: const Text("Reminder"),
                    details: Text(
                      reminderMinutes.value == 0
                          ? "Off"
                          : "${reminderMinutes.value} min before",
                    ),
                    selectControl: .managedRadio(
                      initial: reminderMinutes.value,
                      onChange: loading.value
                          ? null
                          : (value) {
                              if (value.firstOrNull case final minutes?) {
                                reminderMinutes.value = minutes;
                              }
                            },
                    ),
                    menu: [
                      const FSelectTile<int>(title: Text("Off"), value: 0),
                      for (final minutes in const [5, 10, 15, 30, 60])
                        FSelectTile<int>(
                          title: Text("$minutes min before"),
                          value: minutes,
                        ),
                    ],
                  ),
                  // Templates are rarely changed; keep them folded away.
                  FAccordion(
                    children: [
                      FAccordionItem(
                        title: const Text("Customize event text"),
                        initiallyExpanded: false,
                        child: Column(
                          crossAxisAlignment: .stretch,
                          children: [
                            const SizedBox(height: 8),
                            FTextField(
                              control: .managed(
                                controller: titleTemplateController,
                              ),
                              label: const Text("Title Template"),
                              hint: "{name}",
                            ),
                            const SizedBox(height: 8),
                            FTextField.multiline(
                              maxLines: 4,
                              control: .managed(
                                controller: descriptionTemplateController,
                              ),
                              label: const Text("Description Template"),
                              hint: "Course: {courseCode}",
                            ),
                            const SizedBox(height: 8),
                            FTextField(
                              control: .managed(
                                controller: locationTemplateController,
                              ),
                              label: const Text("Location Template"),
                              hint: "{block}-{roomNo}",
                            ),
                            const SizedBox(height: 8),
                            Text(
                              "Placeholders: {name} {courseCode} {courseType} {faculty} {slot} {day} {startTime} {endTime} {block} {roomNo}",
                              style: typography.body.sm,
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            FButton(
              onPress: (loading.value || !canSync) ? null : sync,
              child: loading.value
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
                    crossAxisAlignment: .stretch,
                    children: [
                      Text(
                        "Clear only removes events tagged by this app for the current semester.",
                        style: typography.body.sm,
                        textAlign: .center,
                      ),
                      const SizedBox(height: 8),
                      FButton(
                        variant: .destructive,
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
            const SizedBox(height: 18),
            Text(
              "Note: Calendar apps may take a few minutes to display updates.",
              style: typography.body.sm,
              textAlign: .center,
            ),
          ],
        ),
      ),
    );
  }
}
