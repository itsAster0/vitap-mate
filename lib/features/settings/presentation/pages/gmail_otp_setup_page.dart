import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/email_otp/google_oauth_loopback.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';

const _sharedWarning =
    'Fallback only. Shared Google access may be unavailable because of OAuth user limits, rate limits, verification status, or VIT Workspace policy. Try the personal BYOK method first.';

class GmailOtpSetupPage extends HookConsumerWidget {
  const GmailOtpSetupPage({
    this.androidByokOverride,
    this.sharedOAuthOverride,
    super.key,
  });

  final bool? androidByokOverride;
  final bool? sharedOAuthOverride;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final oauth = ref.read(googleEmailOtpAuthServiceProvider);
    final credentials = useState<GoogleDesktopOAuthCredentials?>(null);
    final currentSession = useState<EmailOtpOAuthSession?>(null);
    final busy = useState(false);
    final statusMessage = useState<String?>(null);
    final byokPoller = useRef<StreamSubscription<EmailOtpOAuthSession>?>(null);
    final byokPollDetected = useRef(false);

    Future<void> refreshSession() async {
      currentSession.value = await oauth.loadSession();
      ref.invalidate(emailOtpReadyProvider);
    }

    useEffect(() {
      unawaited(refreshSession());
      return () {
        unawaited(byokPoller.value?.cancel());
        unawaited(oauth.cancelByokSetup());
      };
    }, const []);

    useEffect(() {
      final listener = AppLifecycleListener(
        onResume: () {
          // The localhost callback may finish while the system browser still
          // covers the app. Refresh more than once so the completed token
          // exchange is reflected as soon as the user returns.
          for (final delay in const [
            Duration.zero,
            Duration(milliseconds: 500),
            Duration(seconds: 2),
          ]) {
            unawaited(
              Future<void>.delayed(delay, () async {
                if (context.mounted) await refreshSession();
              }),
            );
          }
        },
      );
      return listener.dispose;
    }, const []);

    Future<String?> expectedAccountIdentifier() async {
      // VtopClient.username is replaced with the parsed registration number
      // during login. Both personal and shared OAuth must validate against
      // that value rather than the user's portal login name.
      var identifier = '';
      try {
        await ref.read(vClientProvider.notifier).ensureLogin();
        identifier = (await ref.read(vClientProvider.future)).username.trim();
      } catch (_) {
        if (context.mounted) {
          dispToast(
            context,
            'VTOP sign-in required',
            'Check your saved VTOP credentials in Settings, sign in once, then retry Gmail setup.',
          );
        }
        return null;
      }
      if (identifier.isEmpty && context.mounted) {
        dispToast(
          context,
          'Setup failed',
          'Open Vitap Mate once with your VTOP account, then retry Gmail setup.',
        );
      }
      return identifier.isEmpty ? null : identifier;
    }

    Future<void> waitUntilAppIsForeground() async {
      while (context.mounted &&
          WidgetsBinding.instance.lifecycleState != AppLifecycleState.resumed) {
        await Future<void>.delayed(const Duration(milliseconds: 200));
      }
    }

    Future<void> importCredentials() async {
      if (busy.value) return;
      busy.value = true;
      statusMessage.value = null;
      try {
        final picked = await FilePicker.pickFile(
          type: FileType.custom,
          allowedExtensions: const ['json'],
        );
        if (picked == null) return;
        if (await picked.length() > 64 * 1024) {
          throw const GoogleDesktopOAuthException(
            'file_too_large',
            'The OAuth credential file must be smaller than 64 KB.',
          );
        }
        final Uint8List bytes = await picked.readAsBytes();
        credentials.value = oauth.parseByokCredentials(bytes);
        statusMessage.value = 'Desktop OAuth credentials are ready.';
      } on GoogleDesktopOAuthException catch (error) {
        credentials.value = null;
        statusMessage.value = error.message;
      } catch (_) {
        credentials.value = null;
        statusMessage.value =
            'Could not import this file. Select the Desktop OAuth JSON downloaded from Google.';
      } finally {
        busy.value = false;
      }
    }

    Future<void> connectByok() async {
      final selected = credentials.value;
      if (selected == null || busy.value) return;
      final accountIdentifier = await expectedAccountIdentifier();
      if (accountIdentifier == null) return;
      busy.value = true;
      statusMessage.value = 'Waiting for Google in your browser…';
      final previousSession = await oauth.loadSession();
      await byokPoller.value?.cancel();
      byokPollDetected.value = false;
      byokPoller.value = oauth
          .pollForPersonalSession(
            clientId: selected.clientId,
            previousSession: previousSession,
          )
          .listen(
            (session) {
              byokPollDetected.value = true;
              if (!context.mounted) return;
              currentSession.value = session;
              statusMessage.value = 'Personal Gmail access is connected.';
              ref.invalidate(emailOtpReadyProvider);
            },
            onDone: () {
              if (!context.mounted || !busy.value || byokPollDetected.value) {
                return;
              }
              statusMessage.value =
                  'Google setup timed out after five minutes. Retry and keep Vitap Mate running while the browser is open.';
              busy.value = false;
              unawaited(oauth.cancelByokSetup());
            },
          );
      try {
        final result = await oauth.setupByok(
          credentials: selected,
          expectedUsername: accountIdentifier,
          openBrowser: (uri) =>
              launchUrl(uri, mode: LaunchMode.externalApplication),
          beforeTokenExchange: waitUntilAppIsForeground,
          onProgress: (message) {
            if (context.mounted) statusMessage.value = message;
          },
        );
        statusMessage.value = result.message;
        if (result.success) {
          await refreshSession();
          if (context.mounted) {
            dispToast(context, 'Connected', result.message);
          }
        } else if (context.mounted) {
          dispToast(context, 'Gmail not connected', result.message);
        }
      } finally {
        await byokPoller.value?.cancel();
        byokPoller.value = null;
        byokPollDetected.value = false;
        busy.value = false;
      }
    }

    Future<void> connectShared() async {
      if (busy.value) return;
      final confirmed = await showAdaptiveDialog<bool>(
        context: context,
        builder: (dialogContext) => AppDialog(
          title: const Text('Use shared Google access?'),
          body: const Text(_sharedWarning),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(dialogContext).pop(false),
              child: const Text('Cancel'),
            ),
            FButton(
              onPress: () => Navigator.of(dialogContext).pop(true),
              child: const Text('Continue with fallback'),
            ),
          ],
        ),
      );
      if (confirmed != true) return;
      final accountIdentifier = await expectedAccountIdentifier();
      if (accountIdentifier == null) return;
      busy.value = true;
      statusMessage.value = 'Starting shared Google authorization…';
      try {
        final result = await oauth.setupIdentityThenGmail(
          expectedUsername: accountIdentifier,
        );
        statusMessage.value = result.message;
        if (result.success) {
          await refreshSession();
          if (context.mounted) {
            dispToast(context, 'Connected', result.message);
          }
        } else if (context.mounted) {
          dispToast(context, 'Gmail not connected', result.message);
        }
      } finally {
        busy.value = false;
      }
    }

    Future<void> disconnect() async {
      final confirmed = await showAdaptiveDialog<bool>(
        context: context,
        builder: (dialogContext) => AppDialog(
          title: const Text('Disconnect Gmail?'),
          body: const Text(
            'This removes the saved OAuth credentials and Gmail tokens from this device.',
          ),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(dialogContext).pop(false),
              child: const Text('Cancel'),
            ),
            FButton(
              variant: FButtonVariant.destructive,
              onPress: () => Navigator.of(dialogContext).pop(true),
              child: const Text('Disconnect'),
            ),
          ],
        ),
      );
      if (confirmed != true) return;
      busy.value = true;
      try {
        await oauth.clearSession();
        statusMessage.value = 'Gmail access was disconnected.';
        await refreshSession();
      } finally {
        busy.value = false;
      }
    }

    Future<void> testLatestEmail() async {
      if (busy.value) return;
      busy.value = true;
      try {
        final result = await oauth.fetchLatestInfoEmail(
          deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
        );
        if (context.mounted) {
          dispToast(
            context,
            result == null ? 'No OTP email found' : 'Gmail access works',
            result == null
                ? 'No recent VTOP OTP email was found.'
                : 'Latest message: ${result.subject}',
          );
        }
      } catch (error) {
        if (context.mounted) {
          dispToast(context, 'Gmail test failed', '$error');
        }
        await refreshSession();
      } finally {
        busy.value = false;
      }
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 32),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Gmail OTP Setup',
            style: context.theme.typography.body.xl.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 6),
          Text(
            'Connect Gmail directly from this device. Vitap Mate does not send your OAuth credentials or email to a hosted backend.',
            style: context.theme.typography.body.sm.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
          if (currentSession.value case final session?) ...[
            const SizedBox(height: 16),
            FAlert(
              variant: FAlertVariant.primary,
              title: Text('${session.authSourceLabel} connected'),
              subtitle: Text(session.email),
            ),
            const SizedBox(height: 8),
            Row(
              spacing: 8,
              children: [
                Expanded(
                  child: FButton(
                    variant: FButtonVariant.outline,
                    onPress: busy.value ? null : testLatestEmail,
                    child: const Text('Test Gmail'),
                  ),
                ),
                Expanded(
                  child: FButton(
                    variant: FButtonVariant.destructive,
                    onPress: busy.value ? null : disconnect,
                    child: const Text('Disconnect'),
                  ),
                ),
              ],
            ),
          ],
          if (androidByokOverride ?? Platform.isAndroid) ...[
            const SizedBox(height: 20),
            _SectionCard(
              title: 'Use your own Google access',
              badge: 'Preferred',
              children: [
                const Text(
                  'Import a Desktop OAuth JSON file, then connect Gmail directly on this device.',
                ),
                const SizedBox(height: 12),
                FTile(
                  prefix: const Icon(FLucideIcons.bookOpen),
                  title: Text(
                    'How to get OAuth credentials',
                    style: TextStyle(color: context.theme.colors.primary),
                  ),
                  subtitle: const Text('Open the step-by-step guide'),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => context.pushNamed(Paths.gmailOauthGuide),
                ),
                const SizedBox(height: 14),
                FButton(
                  variant: FButtonVariant.outline,
                  onPress: busy.value ? null : importCredentials,
                  child: Text(
                    credentials.value == null
                        ? 'Import OAuth JSON'
                        : 'Replace OAuth JSON',
                  ),
                ),
                if (credentials.value case final selected?) ...[
                  const SizedBox(height: 8),
                  Text(
                    '${selected.projectId} · ${selected.redactedClientId}',
                    style: context.theme.typography.body.xs.copyWith(
                      color: context.theme.colors.mutedForeground,
                    ),
                  ),
                  const SizedBox(height: 8),
                  FButton(
                    onPress: busy.value ? null : connectByok,
                    child: const Text('Connect with Google'),
                  ),
                ],
              ],
            ),
          ] else ...[
            const SizedBox(height: 16),
            const FAlert(
              title: Text('Personal BYOK is Android-only'),
              subtitle: Text(
                'This first release supports local BYOK on Android. Use shared access on this platform.',
              ),
            ),
          ],
          if (sharedOAuthOverride ?? sharedGoogleOAuthEnabled) ...[
            const SizedBox(height: 16),
            _SectionCard(
              title: 'Use shared app access',
              badge: 'Fallback',
              children: [
                const FAlert(
                  variant: FAlertVariant.destructive,
                  title: Text('Shared access may not work'),
                  subtitle: Text(_sharedWarning),
                ),
                const SizedBox(height: 12),
                FButton(
                  variant: FButtonVariant.outline,
                  onPress: busy.value ? null : connectShared,
                  child: const Text('Use shared fallback'),
                ),
              ],
            ),
          ],
          if (statusMessage.value case final message?) ...[
            const SizedBox(height: 14),
            FAlert(
              title: Text(busy.value ? 'Working…' : 'Setup status'),
              subtitle: Text(message),
            ),
          ],
          if (busy.value) ...[
            const SizedBox(height: 14),
            const Center(child: FCircularProgress.pinwheel()),
          ],
        ],
      ),
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.badge,
    required this.children,
  });

  final String title;
  final String badge;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) => FCard(
    child: Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  title,
                  style: context.theme.typography.body.lg.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              FBadge(child: Text(badge)),
            ],
          ),
          const SizedBox(height: 12),
          ...children,
        ],
      ),
    ),
  );
}
