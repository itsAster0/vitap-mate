import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/vtop/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/vtop/email_otp/google_oauth_loopback.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/features/settings/presentation/email_otp_support.dart';
import 'package:vitapmate/features/settings/presentation/widgets/email_otp_widgets.dart';

class _Step {
  const _Step({
    required this.title,
    required this.actions,
    required this.doneWhen,
    this.warning,
    this.link,
    this.badge,
  });

  final String title;
  final List<String> actions;
  final String doneWhen;
  final String? warning;
  final GuideLink? link;
  final String? badge;
}

const _steps = <_Step>[
  _Step(
    title: 'Create a Google Cloud project',
    actions: [
      'Sign in with your personal Gmail (…@gmail.com), not your college account.',
      'Project name: anything, e.g. vitap-mate. Leave Location as “No organization”.',
      'Tap Create, wait a few seconds, then pick the new project in the picker at the top.',
    ],
    doneWhen: 'the project picker at the top shows your new project.',
    link: GuideLink('https://console.cloud.google.com/projectcreate'),
  ),
  _Step(
    title: 'Turn on the Gmail API',
    actions: [
      'Check the project picker at the top still shows your project.',
      'Tap Enable.',
    ],
    doneWhen: 'the page shows “API Enabled” or a Manage button.',
    link: GuideLink(
      'https://console.cloud.google.com/apis/library/gmail.googleapis.com',
    ),
  ),
  _Step(
    title: 'Set up the consent screen',
    actions: [
      'Tap Get started. Already see a filled Branding form? This step is done.',
      'App name “Vitap Mate”, support email = your Gmail. Next.',
      'Audience: External. Next.',
      'Contact: your Gmail. Tick the policy box, then Create.',
    ],
    warning: 'Pick External. Internal only works for Workspace organisations.',
    doneWhen: 'you land on the Google Auth Platform overview.',
    link: GuideLink('https://console.cloud.google.com/auth/branding'),
  ),
  _Step(
    title: 'Add your college email as a test user',
    actions: [
      'Scroll to Test users and tap Add users.',
      'Enter your @vitapstudent.ac.in email and tap Save.',
      'Sharing this project with friends? Add their college emails too.',
    ],
    doneWhen: 'your college email is listed under Test users.',
    link: GuideLink('https://console.cloud.google.com/auth/audience'),
  ),
  _Step(
    title: 'Create a Desktop app client, download the JSON',
    actions: [
      'Tap Create client.',
      'Application type: Desktop app. Name: anything. Tap Create.',
      'In the popup, tap Download JSON.',
      'Used a computer? Move the file to this phone (Drive, Telegram saved messages, USB).',
    ],
    warning:
        'Pick Desktop app, not Android or Web. Download the JSON from the popup right away: Google never shows the secret again. Missed it? Create a new client.',
    doneWhen: 'a client_secret_….json file is in this phone’s Downloads.',
    link: GuideLink('https://console.cloud.google.com/auth/clients'),
  ),
  _Step(
    title: 'Publish the app',
    badge: 'Stops weekly re-login',
    actions: [
      'Under Publishing status, tap Publish app, then Confirm.',
      'Skip the verification prompt. You don’t need it.',
    ],
    warning:
        'While the status is Testing, Google ends the connection every 7 days. If VIT blocks publishing, stay in Testing and reconnect each week.',
    doneWhen: 'Publishing status says “In production”.',
    link: GuideLink('https://console.cloud.google.com/auth/audience'),
  ),
  _Step(
    title: 'Connect in Vitap Mate',
    actions: [
      'Tap Import JSON below and pick the downloaded file.',
      'Tap Connect Google and choose your @vitapstudent.ac.in account.',
      '“Google hasn’t verified this app”: check the app name is yours, then Continue (or Advanced → Go to Vitap Mate).',
      'Tick the Gmail permission, then Continue. Come back when the browser says you can close it.',
    ],
    doneWhen: 'this screen says Gmail is connected.',
  ),
];

const _troubleshooting = <(String, String)>[
  (
    'Error 403: access_denied',
    'Your college email is not a test user (step 4), or you picked the wrong Google account.',
  ),
  (
    '“Use your @vitapstudent.ac.in email”',
    'You chose a personal account in the Google picker. Retry and choose your college account.',
  ),
  (
    '“Create a Desktop app OAuth client”',
    'The JSON is from a Web or Android client. Redo step 5 and choose Desktop app.',
  ),
  (
    'Gmail check fails right after sign-in',
    'The Gmail API is not on in this project (step 2). It can take a minute to apply.',
  ),
  (
    'Worked, then stopped after a week',
    'The project is still in Testing. Do step 6, then reconnect.',
  ),
  (
    'Error 403: org_internal',
    'The audience is Internal. Open Audience and switch it to External (step 3).',
  ),
  (
    'Access blocked by your admin',
    'VIT’s Workspace policy blocks the app. Use email forwarding instead.',
  ),
  (
    'Using a friend’s JSON?',
    'It works and gives them no access to your Gmail, but they must add your college email as a test user unless their app is published. They can revoke it any time.',
  ),
  (
    'Why does the permission sound broad?',
    'gmail.modify lets Vitap Mate read VTOP’s OTP mail and move a used one to Trash. It never deletes mail for good, and your tokens stay on this phone.',
  ),
];

/// Option 2: the student's own Google OAuth client. A guided checklist
/// with the import and connect actions pinned under it.
class GmailOAuthGuidePage extends HookConsumerWidget {
  const GmailOAuthGuidePage({this.androidOverride, super.key});

  final bool? androidOverride;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final app = colors.app;
    final oauth = ref.read(googleEmailOtpAuthServiceProvider);
    final steps = useGuideSteps(ref, byokGuideDoneSettingKey);
    final openStep = useState<int?>(null);
    final credentials = useState<GoogleDesktopOAuthCredentials?>(null);
    final session = useState<EmailOtpOAuthSession?>(null);
    final busy = useState(false);
    final status = useState<String?>(null);
    final poller = useRef<StreamSubscription<EmailOtpOAuthSession>?>(null);
    final pollDetected = useRef(false);
    final current = openStep.value ?? steps.firstOpen(_steps.length);

    Future<void> refreshSession() async {
      session.value = await oauth.loadSession();
      ref.invalidate(emailOtpReadyProvider);
      ref.invalidate(anyEmailOtpReadyProvider);
    }

    useEffect(() {
      unawaited(refreshSession());
      return () {
        unawaited(poller.value?.cancel());
        unawaited(oauth.cancelByokSetup());
      };
    }, const []);

    useEffect(() {
      // The loopback callback may finish while the browser still covers
      // the app; check again a few times after coming back.
      final listener = AppLifecycleListener(
        onResume: () {
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

    Future<void> waitUntilForeground() async {
      while (context.mounted &&
          WidgetsBinding.instance.lifecycleState != AppLifecycleState.resumed) {
        await Future<void>.delayed(const Duration(milliseconds: 200));
      }
    }

    Future<void> importJson() async {
      if (busy.value) return;
      busy.value = true;
      status.value = null;
      try {
        final picked = await FilePicker.pickFile(
          type: FileType.custom,
          allowedExtensions: const ['json'],
        );
        if (picked == null) return;
        if (await picked.length() > 64 * 1024) {
          throw const GoogleDesktopOAuthException(
            'file_too_large',
            'The OAuth file must be smaller than 64 KB.',
          );
        }
        final Uint8List bytes = await picked.readAsBytes();
        credentials.value = oauth.parseByokCredentials(bytes);
        status.value = null;
      } on GoogleDesktopOAuthException catch (error) {
        credentials.value = null;
        status.value = error.message;
      } catch (_) {
        credentials.value = null;
        status.value =
            'Could not read this file. Pick the Desktop app JSON from step 5.';
      } finally {
        busy.value = false;
      }
    }

    Future<void> connect() async {
      final selected = credentials.value;
      if (selected == null || busy.value) return;
      final account = await gmailAccountIdentifier(context, ref);
      if (account == null) return;
      busy.value = true;
      status.value = 'Waiting for Google in your browser…';
      final previous = await oauth.loadSession();
      await poller.value?.cancel();
      pollDetected.value = false;
      poller.value = oauth
          .pollForPersonalSession(
            clientId: selected.clientId,
            previousSession: previous,
          )
          .listen(
            (found) {
              pollDetected.value = true;
              if (!context.mounted) return;
              session.value = found;
              status.value = null;
              ref.invalidate(emailOtpReadyProvider);
            },
            onDone: () {
              if (!context.mounted || !busy.value || pollDetected.value) {
                return;
              }
              status.value =
                  'Google took longer than five minutes. Retry and keep Vitap Mate running while the browser is open.';
              busy.value = false;
              unawaited(oauth.cancelByokSetup());
            },
          );
      try {
        final result = await oauth.setupByok(
          credentials: selected,
          expectedUsername: account,
          openBrowser: (uri) =>
              launchUrl(uri, mode: LaunchMode.externalApplication),
          beforeTokenExchange: waitUntilForeground,
          onProgress: (message) {
            if (context.mounted) status.value = message;
          },
        );
        if (result.success) {
          status.value = null;
          await refreshSession();
          steps.setAll(_steps.length);
          if (context.mounted) dispToast(context, 'Connected', result.message);
        } else {
          status.value = result.message;
          if (context.mounted) {
            dispToast(context, 'Gmail not connected', result.message);
          }
        }
      } finally {
        await poller.value?.cancel();
        poller.value = null;
        pollDetected.value = false;
        busy.value = false;
      }
    }

    final connected = session.value;
    final android = androidOverride ?? Platform.isAndroid;

    Widget bottomBar() {
      if (connected != null) {
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            GuideCallout(
              icon: FLucideIcons.circleCheck,
              tone: app.success,
              child: Text('Gmail connected · ${connected.email}'),
            ),
            const SizedBox(height: Space.sm),
            FButton(onPress: () => context.pop(), child: const Text('Done')),
          ],
        );
      }
      if (!android) {
        return const FAlert(
          title: Text('Your own key is Android-only for now'),
          subtitle: Text('Use email forwarding on this device.'),
        );
      }
      final selected = credentials.value;
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (status.value case final message?) ...[
            Row(
              children: [
                if (busy.value) ...[
                  const SizedBox(
                    width: 14,
                    height: 14,
                    child: FCircularProgress.pinwheel(),
                  ),
                  const SizedBox(width: Space.sm),
                ],
                Expanded(
                  child: Text(
                    message,
                    style: typography.body.xs.copyWith(
                      color: busy.value
                          ? colors.mutedForeground
                          : app.danger.onSubtle,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: Space.sm),
          ],
          Row(
            children: [
              Expanded(
                child: FButton(
                  variant: FButtonVariant.outline,
                  onPress: busy.value ? null : importJson,
                  prefix: const Icon(FLucideIcons.fileJson),
                  child: Text(selected == null ? 'Import JSON' : 'Replace'),
                ),
              ),
              const SizedBox(width: Space.sm),
              Expanded(
                child: FButton(
                  onPress: busy.value || selected == null ? null : connect,
                  child: const Text('Connect Google'),
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.xs + 2),
          Text(
            selected == null
                ? 'Import the JSON from step 5 to connect.'
                : '${selected.projectId} · ${selected.redactedClientId}',
            textAlign: TextAlign.center,
            style: typography.body.xs.copyWith(color: colors.mutedForeground),
          ),
        ],
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: SingleChildScrollView(
            padding: const EdgeInsets.fromLTRB(
              Space.page,
              Space.sm,
              Space.page,
              Space.xl,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'Option 2 · about 10 minutes, once. Vitap Mate then reads the OTP from Gmail on this phone.',
                  style: typography.body.sm.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
                const SizedBox(height: Space.lg),
                GuideProgress(
                  done: steps.done,
                  total: _steps.length,
                  current: current,
                ),
                const SizedBox(height: Space.lg),
                GuideCallout(
                  icon: FLucideIcons.monitor,
                  tone: app.accentTone,
                  child: const Text(
                    'Easiest on a laptop. On this phone, open each link in Chrome and turn on ⋮ → Desktop site. Your personal Gmail owns the project; your college account connects.',
                  ),
                ),
                const SizedBox(height: Space.md),
                for (final (index, step) in _steps.indexed) ...[
                  GuideStepTile(
                    number: index + 1,
                    title: step.title,
                    done: steps.done.contains(index),
                    open: index == current,
                    onTap: () => openStep.value = index,
                    onToggleDone: () {
                      final finishing = !steps.done.contains(index);
                      steps.toggle(index);
                      // Move on to the next open step.
                      openStep.value = finishing ? null : index;
                    },
                    actions: step.actions,
                    warning: step.warning,
                    doneWhen: step.doneWhen,
                    link: step.link,
                    badge: step.badge,
                  ),
                  const SizedBox(height: Space.sm),
                ],
                const SizedBox(height: Space.md),
                const GuideTroubleshooting(items: _troubleshooting),
              ],
            ),
          ),
        ),
        DecoratedBox(
          decoration: BoxDecoration(
            color: colors.background,
            border: Border(top: BorderSide(color: colors.border)),
          ),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(
              Space.page,
              Space.md,
              Space.page,
              Space.md,
            ),
            child: bottomBar(),
          ),
        ),
      ],
    );
  }
}
