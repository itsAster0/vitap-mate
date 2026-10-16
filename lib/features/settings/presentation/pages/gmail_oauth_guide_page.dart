import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

class _Step {
  const _Step({
    required this.title,
    required this.actions,
    required this.doneWhen,
    this.badge,
    this.warning,
    this.url,
    this.linkLabel,
  });

  final String title;
  final String? badge;
  final List<String> actions;
  final String? warning;
  final String doneWhen;
  final String? url;
  final String? linkLabel;
}

const _steps = <_Step>[
  _Step(
    title: 'Create a Google Cloud project',
    actions: [
      'Open the link and sign in with your personal Gmail (…@gmail.com), not your college account.',
      'Project name: anything, e.g. vitap-mate. Leave Location as “No organization”.',
      'Tap Create, wait a few seconds, then pick the new project in the project picker at the top.',
    ],
    doneWhen: 'The project picker at the top shows your new project.',
    url: 'https://console.cloud.google.com/projectcreate',
    linkLabel: 'Create project',
  ),
  _Step(
    title: 'Turn on the Gmail API',
    actions: [
      'Check that the project picker at the top still shows your project.',
      'Tap Enable.',
    ],
    doneWhen: 'The page shows “API Enabled” or a Manage button.',
    url: 'https://console.cloud.google.com/apis/library/gmail.googleapis.com',
    linkLabel: 'Open Gmail API',
  ),
  _Step(
    title: 'Set up the consent screen',
    actions: [
      'Tap Get started. If you see an already filled Branding form instead, this step is done.',
      'App information: App name “Vitap Mate”, support email = your Gmail. Next.',
      'Audience: choose External. Next.',
      'Contact information: your Gmail. Next, tick the policy box, then Create.',
    ],
    warning:
        'Choose External. Internal is only for Workspace organisations and will not work here.',
    doneWhen: 'You land on the Google Auth Platform overview page.',
    url: 'https://console.cloud.google.com/auth/branding',
    linkLabel: 'Open consent screen',
  ),
  _Step(
    title: 'Add your college email as a test user',
    actions: [
      'Scroll to Test users and tap Add users.',
      'Enter your @vitapstudent.ac.in email and tap Save.',
      'Sharing this project with friends? Add their college emails here too.',
    ],
    doneWhen: 'Your college email is listed under Test users.',
    url: 'https://console.cloud.google.com/auth/audience',
    linkLabel: 'Open Audience',
  ),
  _Step(
    title: 'Create a Desktop app client and download the JSON',
    actions: [
      'Tap Create client.',
      'Application type: Desktop app. Name: anything. Tap Create.',
      'In the popup that appears, tap Download JSON.',
      'Move the file to this phone if you used a computer (Drive, Telegram saved messages, USB).',
    ],
    warning:
        'Pick Desktop app, not Android or Web application. Download the JSON from the popup right away: Google will not show the client secret again after you close it. If you missed it, create a new client.',
    doneWhen:
        'A file named like client_secret_….json is in this phone’s Downloads.',
    url: 'https://console.cloud.google.com/auth/clients',
    linkLabel: 'Open Clients',
  ),
  _Step(
    title: 'Publish the app',
    badge: 'Recommended',
    actions: [
      'Under Publishing status, tap Publish app, then Confirm.',
      'You do not need to submit for verification. Ignore that prompt.',
    ],
    warning:
        'While the status is Testing, Google ends the connection every 7 days and OTP autofill stops until you reconnect. Publishing stops that. If Google or VIT blocks publishing, stay in Testing and reconnect each week.',
    doneWhen: 'Publishing status says “In production”.',
    url: 'https://console.cloud.google.com/auth/audience',
    linkLabel: 'Open Audience',
  ),
  _Step(
    title: 'Connect in Vitap Mate',
    actions: [
      'Go back to Gmail OTP Setup, tap Import OAuth JSON and pick the downloaded file.',
      'Tap Connect with Google and choose your @vitapstudent.ac.in account.',
      '“Google hasn’t verified this app”: check the app name is yours (or your trusted friend’s), then tap Continue (or Advanced → Go to Vitap Mate).',
      'Make sure the Gmail permission is ticked, then tap Continue.',
      'Come back to Vitap Mate when the browser says you can close it.',
    ],
    doneWhen:
        'Gmail OTP Setup shows “Personal BYOK connected”. Tap Test Gmail to confirm.',
  ),
];

const _troubleshooting = <(String, String)>[
  (
    'Error 403: access_denied',
    'Your college email is not a test user (step 4), or you picked the wrong Google account.',
  ),
  (
    '“Use your @vitapstudent.ac.in email”',
    'You signed in with a personal account in the Google picker. Retry and choose your college account.',
  ),
  (
    '“Create a Desktop app OAuth client”',
    'The JSON is from a Web or Android client. Redo step 5 and choose Desktop app.',
  ),
  (
    'Gmail check fails right after sign-in',
    'The Gmail API is not enabled in this project (step 2). It can take a minute to apply after enabling.',
  ),
  (
    'Worked, then stopped after a week',
    'The project is still in Testing. Do step 6, then reconnect.',
  ),
  (
    'Error 403: org_internal',
    'The audience is set to Internal. Open Audience and switch it to External (step 3).',
  ),
  (
    'Access blocked by your admin / admin_policy_enforced',
    'VIT’s Workspace policy is blocking the app. Vitap Mate cannot work around this; use the shared fallback.',
  ),
];

class GmailOAuthGuidePage extends StatelessWidget {
  const GmailOAuthGuidePage({super.key});

  Future<void> _openExternal(BuildContext context, String rawUrl) async {
    final opened = await launchUrl(
      Uri.parse(rawUrl),
      mode: LaunchMode.externalApplication,
    );
    if (!opened && context.mounted) {
      dispToast(context, 'Could not open link', 'Try again in your browser.');
    }
  }

  @override
  Widget build(BuildContext context) {
    final muted = context.theme.typography.body.sm.copyWith(
      color: context.theme.colors.mutedForeground,
    );
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(12, 8, 12, 32),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Get your Google OAuth file',
            style: context.theme.typography.body.xl.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 6),
          Text(
            'You will create a free Google Cloud project, download one Desktop app JSON file and import it on Gmail OTP Setup. This is an OAuth client, not an API key. About 5 minutes, no card needed.',
            style: muted,
          ),
          const SizedBox(height: 16),
          const FAlert(
            title: Text('Before you start'),
            subtitle: Text(
              'Two accounts are involved. Your personal Gmail owns the Cloud project. Your @vitapstudent.ac.in account is the one that connects. College accounts usually cannot create Cloud projects. A laptop is easier: the Cloud console is cramped on a phone.',
            ),
          ),
          const SizedBox(height: 16),
          for (var index = 0; index < _steps.length; index++) ...[
            _GuideStep(
              number: '${index + 1}',
              step: _steps[index],
              onOpen: _steps[index].url == null
                  ? null
                  : () => _openExternal(context, _steps[index].url!),
            ),
            if (index != _steps.length - 1) const SizedBox(height: 14),
          ],
          const SizedBox(height: 20),
          _Troubleshooting(),
          const SizedBox(height: 16),
          const FAlert(
            title: Text('Using a trusted friend’s JSON'),
            subtitle: Text(
              'This works and does not give your friend access to your Gmail, but they must add your college email as a test user (step 4) unless their app is published. They control the client and can revoke it at any time. Never share your access token or refresh token.',
            ),
          ),
          const SizedBox(height: 16),
          const FAlert(
            title: Text('Why the permission sounds broad'),
            subtitle: Text(
              'Vitap Mate requests gmail.modify so it can read VTOP OTP emails and move a used OTP email to Trash when Delete After Reading is on. It never permanently deletes mail, and your tokens stay on this device.',
            ),
          ),
          const SizedBox(height: 16),
          Text(
            'Android note: this uses a Desktop OAuth loopback redirect, which may fail on some devices. Keep Vitap Mate running in the background while the browser is open.',
            style: context.theme.typography.body.xs.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
        ],
      ),
    );
  }
}

class _GuideStep extends StatelessWidget {
  const _GuideStep({
    required this.number,
    required this.step,
    required this.onOpen,
  });

  final String number;
  final _Step step;
  final VoidCallback? onOpen;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final small = context.theme.typography.body.sm;
    return FCard(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  width: 32,
                  height: 32,
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: colors.primary.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(999),
                  ),
                  child: Text(
                    number,
                    style: small.copyWith(
                      color: colors.primary,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Text(
                      step.title,
                      style: context.theme.typography.body.lg.copyWith(
                        fontWeight: FontWeight.w700,
                        height: 1.25,
                      ),
                    ),
                  ),
                ),
                if (step.badge case final badge?) ...[
                  const SizedBox(width: 8),
                  FBadge(child: Text(badge)),
                ],
              ],
            ),
            const SizedBox(height: 12),
            for (final (index, action) in step.actions.indexed)
              Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 20,
                      child: Text(
                        '${String.fromCharCode(97 + index)}.',
                        style: small.copyWith(color: colors.mutedForeground),
                      ),
                    ),
                    Expanded(child: Text(action, style: small)),
                  ],
                ),
              ),
            if (step.warning case final warning?) ...[
              const SizedBox(height: 6),
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: colors.destructive.withValues(alpha: 0.08),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Icon(
                      FLucideIcons.triangleAlert,
                      size: 16,
                      color: colors.destructive,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        warning,
                        style: context.theme.typography.body.xs.copyWith(
                          color: colors.foreground,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
            const SizedBox(height: 10),
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(FLucideIcons.circleCheck, size: 16, color: colors.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Done when: ${step.doneWhen}',
                    style: context.theme.typography.body.xs.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                ),
              ],
            ),
            if (onOpen != null) ...[
              const SizedBox(height: 14),
              FButton(
                size: FButtonSizeVariant.sm,
                variant: FButtonVariant.outline,
                onPress: onOpen,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(step.linkLabel ?? 'Open Google Cloud'),
                    const SizedBox(width: 8),
                    const Icon(FLucideIcons.externalLink, size: 16),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _Troubleshooting extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final typography = context.theme.typography;
    return FCard(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'If something goes wrong',
              style: typography.body.lg.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 10),
            for (final (index, (symptom, fix)) in _troubleshooting.indexed) ...[
              if (index != 0) const FDivider(),
              Text(
                symptom,
                style: typography.body.sm.copyWith(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 2),
              Text(
                fix,
                style: typography.body.xs.copyWith(
                  color: context.theme.colors.mutedForeground,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
