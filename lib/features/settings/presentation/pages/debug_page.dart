import 'dart:async';
import 'dart:developer' show log;
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/clinet_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/app/router/paths.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/http/cronet_net_log.dart';
import 'package:vitapmate/vtop/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/platform/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/attendance/logic/attendance_history.dart';

/// Settings → Debug: what testers and developers use to see why data is
/// wrong, slow or heavy, and to put the app in odd states on purpose.
/// Reached only after unlocking developer tools (long-press Appearance).
class DebugPage extends HookConsumerWidget {
  const DebugPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final reuseTtl = ref.watch(vtopSessionReuseTtlProvider);
    final gmailReady = ref.watch(emailOtpReadyProvider).value ?? false;
    final otpTestBusy = useState(false);
    final netLogRecording = useValueListenable(CronetNetLog.recording);

    FTile tile(
      String title,
      IconData icon,
      VoidCallback onPress, {
      String? subtitle,
      Widget? suffix,
    }) => FTile(
      prefix: Icon(icon),
      title: Text(title),
      subtitle: subtitle == null ? null : Text(subtitle),
      suffix: suffix ?? const Icon(FLucideIcons.chevronRight),
      onPress: onPress,
    );

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(Space.md, 0, Space.md, Space.xxl),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SectionHeader(title: 'What happened'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              tile(
                'Logs',
                Icons.receipt_long_outlined,
                () => context.pushNamed(Paths.debugLogs),
                subtitle: 'The app\'s recent log lines',
              ),
            ],
          ),
          if (CronetNetLog.available) ...[
            const SectionHeader(title: 'Network'),
            FTileGroup(
              children: [
                tile(
                  netLogRecording ? 'Stop and Save Net Log' : 'Record Net Log',
                  FLucideIcons.activity,
                  () => _toggleNetLog(context),
                  subtitle: netLogRecording
                      ? 'Recording: use the app, then tap to save'
                      : 'Why requests use HTTP/2 or HTTP/3',
                ),
                tile(
                  'Forget QUIC Failures',
                  FLucideIcons.rotateCcw,
                  () => _forgetQuicFailures(context),
                  subtitle: 'Try HTTP/3 again from the next launch',
                ),
              ],
            ),
          ],
          const SectionHeader(title: 'Session'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              tile(
                'VTOP Session Reuse',
                FLucideIcons.timer,
                () => _openVtopSessionReuseTtlDialog(context, ref),
                subtitle:
                    'Reuse saved cookies for ${reuseTtl.inMinutes} minutes',
              ),
              tile(
                'Copy Saved Cookies',
                FLucideIcons.copy,
                () => _copySavedCookies(context, ref),
              ),
              tile(
                'Clear Saved Cookies',
                FLucideIcons.trash2,
                () => _clearSavedCookies(context, ref),
              ),
              tile(
                'Copy Token',
                FLucideIcons.radio,
                () => _copyFcmToken(context),
                subtitle: 'This install\'s push token',
              ),
            ],
          ),
          const SectionHeader(title: 'Saved data'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              tile(
                'Drop Latest History Entry',
                FLucideIcons.calendarX,
                () => _dropLatestHistoryEntries(context, ref),
                subtitle: 'Every course, saved until the next refresh',
              ),
              tile(
                'Clear Attendance',
                FLucideIcons.clipboardX,
                () => _clearAttendance(context, ref),
                subtitle: 'Show it as not posted, until the next refresh',
              ),
            ],
          ),
          if (gmailReady) ...[
            const SectionHeader(title: 'Email OTP'),
            FTileGroup(
              children: [
                tile(
                  'Test Latest OTP Email',
                  FLucideIcons.mailCheck,
                  () async {
                    if (otpTestBusy.value) return;
                    otpTestBusy.value = true;
                    try {
                      await _testLatestInfoEmail(context, ref);
                    } finally {
                      otpTestBusy.value = false;
                    }
                  },
                  subtitle: 'Fetch the latest VTOP OTP email',
                  suffix: otpTestBusy.value
                      ? const FCircularProgress.pinwheel()
                      : Icon(
                          FLucideIcons.chevronRight,
                          color: colors.mutedForeground,
                        ),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

/// Starts Cronet's net log, or stops it and saves the file where the user
/// picks.
Future<void> _forgetQuicFailures(BuildContext context) async {
  await forgetQuicFailuresOnNextLaunch();
  if (!context.mounted) return;
  dispToast(context, 'Done', 'Close and reopen the app to try HTTP/3 again.');
}

Future<void> _toggleNetLog(BuildContext context) async {
  try {
    if (!CronetNetLog.recording.value) {
      await CronetNetLog.start();
      if (context.mounted) {
        dispToast(
          context,
          'Recording',
          'Reproduce the issue, then come back to save the log.',
        );
      }
      return;
    }
    final bytes = await CronetNetLog.stop();
    if (bytes == null || bytes.isEmpty) {
      if (context.mounted) {
        dispToast(context, 'No Log', 'Cronet wrote nothing to save.');
      }
      return;
    }
    final stamp = DateTime.now()
        .toIso8601String()
        .split('.')
        .first
        .replaceAll(':', '-');
    final saved = await FilePicker.saveFile(
      fileName: 'cronet-netlog-$stamp.json',
      bytes: bytes,
      mimeType: 'application/json',
    );
    if (saved != null && context.mounted) {
      dispToast(context, 'Saved', 'Open it at netlog-viewer.appspot.com.');
    }
  } catch (error, stackTrace) {
    log(
      'Cronet net log failed',
      name: 'settings.netlog',
      error: error,
      stackTrace: stackTrace,
    );
    if (context.mounted) disCommonToast(context, error);
  }
}

Future<void> _copySavedCookies(BuildContext context, WidgetRef ref) async {
  try {
    final user = await ref.read(vtopUserProvider.future);
    final username = user.username;
    if (username == null || username.isEmpty) {
      if (context.mounted) {
        dispToast(context, "No Account", "Sign in first to copy cookies.");
      }
      return;
    }

    final stored = await loadStoredVtopSession(username);
    final cookies = stored?.snapshot.cookies?.trim() ?? '';
    if (cookies.isEmpty) {
      if (context.mounted) {
        dispToast(
          context,
          "No Saved Cookies",
          "No saved session cookies found. Refresh data once and try again.",
        );
      }
      return;
    }

    final cookieEditorJson = cookieEditorJsonFromHeader(cookies);
    if (cookieEditorJson == '[]') {
      if (context.mounted) {
        dispToast(
          context,
          "Invalid Cookie Data",
          "Saved cookie header could not be converted for Cookie-Editor.",
        );
      }
      return;
    }

    await Clipboard.setData(ClipboardData(text: cookieEditorJson));
    if (context.mounted) {
      dispToast(context, "Copied", "Cookie-Editor JSON copied to clipboard.");
    }
  } catch (error, stackTrace) {
    log(
      'Failed to copy saved cookies',
      name: 'settings',
      error: error,
      stackTrace: stackTrace,
    );
    if (context.mounted) {
      dispToast(context, "Failed", "Could not copy cookies right now.");
    }
  }
}

Future<void> _copyFcmToken(BuildContext context) async {
  try {
    final token = await getFcmTokenForCopy();
    if (token == null || token.trim().isEmpty) {
      if (context.mounted) {
        dispToast(context, "No Token", "Could not get your Token right now.");
      }
      return;
    }

    await Clipboard.setData(ClipboardData(text: token));
    if (context.mounted) {
      dispToast(context, "Copied", "Token copied to clipboard.");
    }
  } catch (error, stackTrace) {
    log(
      'Failed to copy FCM token',
      name: 'settings.fcm',
      error: error,
      stackTrace: stackTrace,
    );
    if (context.mounted) {
      dispToast(context, "Failed", "Could not copy your Token right now.");
    }
  }
}

/// Developer tool: removes the newest row from every course's saved
/// history, so it disagrees with the summary. A refresh restores it.
Future<void> _dropLatestHistoryEntries(
  BuildContext context,
  WidgetRef ref,
) async {
  try {
    final semester = await ref.read(selectedSemesterProvider.future);
    final attendance = await readSavedPage(
      ref.read,
      VtopQueries.attendance(semester),
    );
    var dropped = 0;
    for (final course in attendance?.records ?? <AttendanceRecord>[]) {
      final query = VtopQueries.fullAttendance(
        semester,
        courseId: course.courseId,
        courseType: course.courseType,
      );
      final history = await readSavedPage(ref.read, query);
      if (history == null) continue;
      final trimmed = withoutLatestEntry(history);
      if (trimmed.records.length == history.records.length) continue;
      await writeSavedPage(ref.read, query, trimmed);
      dropped++;
      ref.invalidate(
        cachedFullAttendanceProvider(course.courseType, course.courseId),
      );
      ref.invalidate(
        fullAttendanceProvider(course.courseType, course.courseId),
      );
    }
    if (context.mounted) {
      dispToast(
        context,
        'History Trimmed',
        'Dropped the latest entry from $dropped courses.',
      );
    }
  } catch (e) {
    if (context.mounted) disCommonToast(context, e);
  }
}

/// Developer tool: saves the attendance summary with no courses, as before
/// VTOP posts any. A refresh restores it.
Future<void> _clearAttendance(BuildContext context, WidgetRef ref) async {
  try {
    final query = VtopQueries.attendance(
      await ref.read(selectedSemesterProvider.future),
    );
    final attendance = await readSavedPage(ref.read, query);
    if (attendance != null) {
      await writeSavedPage(
        ref.read,
        query,
        attendance.copyWith(records: const []),
      );
    }
    ref.invalidate(attendanceProvider);
    if (context.mounted) {
      dispToast(context, 'Attendance Cleared', 'Refresh to bring it back.');
    }
  } catch (e) {
    if (context.mounted) disCommonToast(context, e);
  }
}

Future<void> _clearSavedCookies(BuildContext context, WidgetRef ref) async {
  try {
    final user = await ref.read(vtopUserProvider.future);
    final username = user.username;
    if (username == null || username.isEmpty) {
      if (context.mounted) {
        dispToast(context, "No Account", "Sign in first to clear cookies.");
      }
      return;
    }

    if (!context.mounted) return;
    if (!await confirm(
      context,
      title: 'Clear Saved Cookies?',
      body:
          'This removes the saved VTOP session cookies for your account. You may need to sign in again.',
      action: 'Clear',
      destructive: true,
    )) {
      return;
    }

    await clearStoredVtopSession(username);
    // The client keeps its cookies in memory and saves them back on the
    // next request; a new one starts signed out.
    ref.invalidate(vClientProvider);
    if (context.mounted) {
      dispToast(context, "Cleared", "Saved VTOP cookies were cleared.");
    }
  } catch (error, stackTrace) {
    log(
      'Failed to clear saved cookies',
      name: 'settings',
      error: error,
      stackTrace: stackTrace,
    );
    if (context.mounted) {
      dispToast(context, "Failed", "Could not clear cookies right now.");
    }
  }
}

Future<void> _openVtopSessionReuseTtlDialog(
  BuildContext context,
  WidgetRef ref,
) async {
  final currentTtl = ref.read(vtopSessionReuseTtlProvider);

  await showAdaptiveDialog<void>(
    context: context,
    barrierDismissible: true,
    builder: (dialogContext) {
      return HookBuilder(
        builder: (dialogContext) {
          final controller = useTextEditingController(
            text: currentTtl.inMinutes.toString(),
          );
          final errorText = useState<String?>(null);

          return AppDialog(
            title: const Text('VTOP Session Reuse'),
            body: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Enter how long saved cookies can be reused.'),
                const SizedBox(height: 8),
                FTextField(
                  control: FTextFieldControl.managed(controller: controller),
                  keyboardType: TextInputType.number,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                ),
                if (errorText.value != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    errorText.value!,
                    style: dialogContext.theme.typography.body.sm.copyWith(
                      color: dialogContext.theme.colors.destructive,
                    ),
                  ),
                ],
              ],
            ),
            actions: [
              FButton(
                variant: FButtonVariant.outline,
                onPress: () => Navigator.of(dialogContext).pop(),
                child: const Text('Cancel'),
              ),
              FButton(
                onPress: () async {
                  final minutes = int.tryParse(controller.text.trim());
                  if (minutes == null || minutes <= 0) {
                    errorText.value = 'Enter minutes greater than 0.';
                    return;
                  }

                  await setVtopSessionReuseTtl(ref, Duration(minutes: minutes));
                  if (!dialogContext.mounted) return;
                  Navigator.of(dialogContext).pop();
                  if (context.mounted) {
                    dispToast(
                      context,
                      'Saved',
                      'VTOP sessions will be reused for $minutes minutes.',
                    );
                  }
                },
                child: const Text('Save'),
              ),
            ],
          );
        },
      );
    },
  );
}

Future<void> _testLatestInfoEmail(BuildContext context, WidgetRef ref) async {
  try {
    final latest = await ref
        .read(googleEmailOtpAuthServiceProvider)
        .fetchLatestInfoEmail(
          deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
        );
    if (!context.mounted) return;

    await showAdaptiveDialog<void>(
      context: context,
      builder: (dialogContext) {
        if (latest == null) {
          return AppDialog(
            title: const Text('Latest OTP Email'),
            body: const Text('No emails from info1@vitap.ac.in were found.'),
            actions: [
              FButton(
                onPress: () => Navigator.of(dialogContext).pop(),
                child: const Text('Close'),
              ),
            ],
          );
        }

        final localTime = latest.receivedAt.toLocal().toString();
        return AppDialog(
          title: const Text('Latest OTP Email'),
          body: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Subject: ${latest.subject}'),
              const SizedBox(height: 6),
              Text('Received: $localTime'),
              const SizedBox(height: 6),
              Text('OTP: ${latest.otp ?? 'Not found'}'),
              const SizedBox(height: 12),
              Text(latest.snippet),
            ],
          ),
          actions: [
            FButton(
              onPress: () => Navigator.of(dialogContext).pop(),
              child: const Text('Close'),
            ),
          ],
        );
      },
    );
  } catch (error, stackTrace) {
    log(
      'Failed to fetch latest info1 email',
      name: 'settings.email_otp',
      error: error,
      stackTrace: stackTrace,
    );
    if (context.mounted) {
      dispToast(context, "Test Failed", "$error");
    }
  }
}
