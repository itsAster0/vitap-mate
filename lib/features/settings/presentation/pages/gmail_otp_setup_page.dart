import 'dart:async';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/theme/app_palette.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/features/settings/presentation/email_otp_support.dart';
import 'package:vitapmate/features/settings/presentation/widgets/email_otp_widgets.dart';

const _sharedWarning =
    'Last resort. Shared Google access may stop working because of Google’s user limits, rate limits, verification status or VIT Workspace policy. Try forwarding or your own key first.';

/// The Email OTP screen: picks how Vitap Mate reads VTOP's OTP mail.
/// Option 1 is email forwarding, option 2 the student's own Google key,
/// and shared Google access (only in builds with its key) the last resort.
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
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final app = colors.app;
    final oauth = ref.read(googleEmailOtpAuthServiceProvider);
    final session = useState<EmailOtpOAuthSession?>(null);
    final forwarding = ref.watch(mailForwardingProvider).value;
    final busy = useState(false);

    Future<void> refresh() async {
      session.value = await oauth.loadSession();
      ref.invalidate(emailOtpReadyProvider);
      ref.invalidate(anyEmailOtpReadyProvider);
      ref.invalidate(mailForwardingProvider);
    }

    useEffect(() {
      unawaited(refresh());
      final listener = AppLifecycleListener(
        onResume: () => unawaited(refresh()),
      );
      return listener.dispose;
    }, const []);

    Future<void> openRoute(String name) async {
      await context.pushNamed(name);
      if (context.mounted) await refresh();
    }

    Future<void> connectShared() async {
      if (busy.value) return;
      final confirmed = await confirm(
        context,
        title: 'Use shared Google access?',
        body: _sharedWarning,
        action: 'Continue',
      );
      if (!confirmed || !context.mounted) return;
      final account = await gmailAccountIdentifier(context, ref);
      if (account == null) return;
      busy.value = true;
      try {
        final result = await oauth.setupIdentityThenGmail(
          expectedUsername: account,
        );
        await refresh();
        if (context.mounted) {
          dispToast(
            context,
            result.success ? 'Connected' : 'Gmail not connected',
            result.message,
          );
        }
      } finally {
        busy.value = false;
      }
    }

    Future<void> disconnect() async {
      final confirmed = await confirm(
        context,
        title: 'Disconnect Gmail?',
        body:
            'This removes the saved Google key and Gmail tokens from this phone.',
        action: 'Disconnect',
        destructive: true,
      );
      if (!confirmed) return;
      busy.value = true;
      try {
        await oauth.clearSession();
        await refresh();
      } finally {
        busy.value = false;
      }
    }

    Future<void> testGmail() async {
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
        if (context.mounted) dispToast(context, 'Gmail test failed', '$error');
        await refresh();
      } finally {
        busy.value = false;
      }
    }

    final gmail = session.value;
    final ownKey = gmail?.authSource == EmailOtpAuthSource.personalByok
        ? gmail
        : null;
    final shared = gmail?.authSource == EmailOtpAuthSource.sharedBuiltIn
        ? gmail
        : null;
    final android = androidByokOverride ?? Platform.isAndroid;

    Widget connectedLine(String text) => Row(
      children: [
        Icon(FLucideIcons.circleCheck, size: 16, color: app.success.base),
        const SizedBox(width: 6),
        Expanded(
          child: Text(
            text,
            style: typography.body.xs.copyWith(
              color: app.success.onSubtle,
              fontWeight: FontWeight.w500,
            ),
          ),
        ),
      ],
    );

    final (statusTone, statusTitle, statusText) = switch ((gmail, forwarding)) {
      (final g?, _) => (
        app.success,
        'Set up',
        'OTPs are read from Gmail (${g.email}).',
      ),
      (null, final f?) => (
        app.success,
        'Set up',
        f.lastOtpAt == null
            ? 'Forwarding address ready. Finish the Gmail filter if you haven’t.'
            : 'OTPs arrive through forwarding.',
      ),
      _ => (
        app.warning,
        'Not set up',
        'VTOP asks for an OTP at sign-in. Pick one way for Vitap Mate to read it, so you never type it.',
      ),
    };

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        Space.page,
        Space.sm,
        Space.page,
        Space.xl,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          GuideCallout(
            icon: gmail != null || forwarding != null
                ? FLucideIcons.circleCheck
                : FLucideIcons.mail,
            tone: statusTone,
            child: Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: '$statusTitle · ',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  TextSpan(text: statusText),
                ],
              ),
            ),
          ),
          const SizedBox(height: Space.lg),
          OtpMethodCard(
            highlighted: ownKey == null && forwarding == null,
            icon: FLucideIcons.forward,
            iconTone: app.lab,
            eyebrow: 'OPTION 1 · RECOMMENDED',
            title: 'Email forwarding',
            description:
                'Gmail forwards only VTOP’s OTP mail to your private address. The code reaches this phone sealed; our server can’t read it.',
            facts: const [
              ('One-time setup', '~2 min'),
              ('Who sees mail', 'Mail relay, briefly'),
            ],
            points: const [
              MethodPoint('No Google Cloud project'),
              MethodPoint('Fills in instantly, even with the app closed'),
              MethodPoint('Lets the server sign in for you, if you turn it on'),
            ],
            status: forwarding == null
                ? null
                : connectedLine(
                    forwarding.lastOtpAt == null
                        ? 'Address created · waiting for the first OTP'
                        : 'Working · ${forwarding.address}',
                  ),
            action: FButton(
              variant: ownKey == null && forwarding == null
                  ? FButtonVariant.primary
                  : FButtonVariant.outline,
              onPress: () => openRoute(Paths.mailForwarding),
              suffix: const Icon(FLucideIcons.arrowRight),
              child: Text(
                forwarding == null ? 'Set up forwarding' : 'Manage forwarding',
              ),
            ),
          ),
          const SizedBox(height: Space.md),
          OtpMethodCard(
            icon: FLucideIcons.keyRound,
            iconTone: app.accentTone,
            eyebrow: 'OPTION 2 · MOST CONTROL',
            title: 'Your own Google key',
            description:
                'Make a free Google Cloud key once. Vitap Mate reads the OTP straight from Gmail; nothing passes through our server.',
            facts: const [
              ('One-time setup', '~10 min'),
              ('Who sees mail', 'Only this phone'),
            ],
            points: const [
              MethodPoint('Can move used OTP mail to Trash'),
              MethodPoint(
                'Only this phone; server sign-in needs forwarding',
                caveat: true,
              ),
              MethodPoint(
                'Needs a laptop, or Chrome’s desktop mode',
                caveat: true,
              ),
            ],
            status: ownKey == null
                ? null
                : connectedLine('Connected · ${ownKey.email}'),
            action: ownKey != null
                ? Row(
                    children: [
                      Expanded(
                        child: FButton(
                          variant: FButtonVariant.outline,
                          size: FButtonSizeVariant.sm,
                          onPress: busy.value ? null : testGmail,
                          child: const Text('Test Gmail'),
                        ),
                      ),
                      const SizedBox(width: Space.sm),
                      Expanded(
                        child: FButton(
                          variant: FButtonVariant.outline,
                          size: FButtonSizeVariant.sm,
                          onPress: busy.value ? null : disconnect,
                          child: Text(
                            'Disconnect',
                            style: TextStyle(color: app.danger.base),
                          ),
                        ),
                      ),
                    ],
                  )
                : android
                ? FButton(
                    variant: FButtonVariant.outline,
                    onPress: () => openRoute(Paths.gmailOauthGuide),
                    suffix: const Icon(FLucideIcons.arrowRight),
                    child: const Text('Start the guide'),
                  )
                : const FAlert(
                    title: Text('Android only for now'),
                    subtitle: Text('Use email forwarding on this device.'),
                  ),
          ),
          if (ownKey != null && forwarding != null) ...[
            const SizedBox(height: Space.sm),
            Text(
              'Both are on. Vitap Mate waits for the forwarded code first and falls back to Gmail.',
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
          ],
          if (sharedOAuthOverride ?? sharedGoogleOAuthEnabled) ...[
            const SizedBox(height: Space.lg),
            FTileGroup(
              children: [
                FTile(
                  prefix: Icon(
                    FLucideIcons.users,
                    color: colors.mutedForeground,
                  ),
                  title: const Text('Shared Google access'),
                  subtitle: Text(
                    shared != null
                        ? 'Connected · ${shared.email}'
                        : 'Last resort · may stop working',
                  ),
                  suffix: busy.value
                      ? const FCircularProgress.pinwheel()
                      : shared != null
                      ? FButton(
                          variant: FButtonVariant.ghost,
                          size: FButtonSizeVariant.sm,
                          mainAxisSize: MainAxisSize.min,
                          onPress: disconnect,
                          child: const Text('Disconnect'),
                        )
                      : const Icon(FLucideIcons.chevronRight),
                  onPress: busy.value || shared != null ? null : connectShared,
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}
