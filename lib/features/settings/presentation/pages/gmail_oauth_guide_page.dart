import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

const _guideSteps = <(String, String, String, String)>[
  (
    '1',
    'Create or select a Google Cloud project',
    'Sign in with a normal Google account. Create a small personal project, or select a project you control. Keep that project selected for every remaining step.',
    'https://console.cloud.google.com/projectcreate',
  ),
  (
    '2',
    'Enable the Gmail API',
    'Check the project selector at the top of Google Cloud, then click Enable for the Gmail API. Wait until the API overview page appears.',
    'https://console.cloud.google.com/apis/library/gmail.googleapis.com',
  ),
  (
    '3',
    'Configure app branding',
    'Enter an app name such as Vitap Mate, choose your support email, and save the required contact information. This is the name Google shows during consent.',
    'https://console.cloud.google.com/auth/branding',
  ),
  (
    '4',
    'Set the audience and test users',
    'Choose External—not Internal—and keep the publishing status as Testing. Under Test users, add every @vitapstudent.ac.in email that will connect. When using a friend’s project, the friend must add your college email here.',
    'https://console.cloud.google.com/auth/audience',
  ),
  (
    '5',
    'Create a Desktop app OAuth client',
    'Choose Create client, select Desktop app as the application type, give it any clear name, and create it. Download the JSON file; do not choose Android or Web application.',
    'https://console.cloud.google.com/auth/clients',
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
  Widget build(BuildContext context) => SingleChildScrollView(
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
          'This creates OAuth credentials, not an API key. Download the Desktop app JSON file and import it on the Gmail OTP Setup page.',
          style: context.theme.typography.body.sm.copyWith(
            color: context.theme.colors.mutedForeground,
          ),
        ),
        const SizedBox(height: 16),
        const FAlert(
          title: Text('Before you start'),
          subtitle: Text(
            'Use a normal Google account to own the Cloud project. Your VITAP college account is added later as a test user.',
          ),
        ),
        const SizedBox(height: 16),
        for (var index = 0; index < _guideSteps.length; index++) ...[
          _GuideStep(
            number: _guideSteps[index].$1,
            title: _guideSteps[index].$2,
            explanation: _guideSteps[index].$3,
            onTap: () => _openExternal(context, _guideSteps[index].$4),
          ),
          if (index != _guideSteps.length - 1) const SizedBox(height: 14),
        ],
        const SizedBox(height: 16),
        const FAlert(
          variant: FAlertVariant.primary,
          title: Text('Return to Vitap Mate'),
          subtitle: Text(
            'Open Gmail OTP Setup, tap Import OAuth JSON, choose the downloaded Desktop app file, and then tap Connect with Google.',
          ),
        ),
        const SizedBox(height: 16),
        const FAlert(
          variant: FAlertVariant.primary,
          title: Text('Using a trusted friend’s JSON'),
          subtitle: Text(
            'This can work, and it does not give your friend your Gmail access. Their Google Cloud project must add your college email as a test user. Only use credentials from someone you trust because they control the OAuth client and can revoke it or view its aggregate usage. Never share your access token or refresh token.',
          ),
        ),
        const SizedBox(height: 16),
        const FAlert(
          variant: FAlertVariant.destructive,
          title: Text('What Google will show'),
          subtitle: Text(
            'A personal Testing project normally shows “Google hasn’t verified this app.” Check that the developer/project name is yours or your trusted friend’s, tap Continue, keep the Gmail permission selected, then tap Continue again. Testing authorizations expire after seven days, so you may need to reconnect.',
          ),
        ),
        const SizedBox(height: 16),
        const FAlert(
          title: Text('Why the permission sounds broad'),
          subtitle: Text(
            'Vitap Mate requests gmail.modify so it can find VTOP OTP emails and move a read OTP message to Trash when Delete After Reading is enabled. It does not permanently delete mail.',
          ),
        ),
        const SizedBox(height: 16),
        Text(
          'Android note: this setup uses a Desktop OAuth loopback redirect on Android. Google documents loopback redirects for Desktop clients, so this experimental method may fail on some devices or Workspace accounts.',
          style: context.theme.typography.body.xs.copyWith(
            color: context.theme.colors.mutedForeground,
          ),
        ),
      ],
    ),
  );
}

class _GuideStep extends StatelessWidget {
  const _GuideStep({
    required this.number,
    required this.title,
    required this.explanation,
    required this.onTap,
  });

  final String number;
  final String title;
  final String explanation;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => FCard(
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
                  color: context.theme.colors.primary.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  number,
                  style: context.theme.typography.body.sm.copyWith(
                    color: context.theme.colors.primary,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  style: context.theme.typography.body.lg.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Text(
            explanation,
            style: context.theme.typography.body.sm.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 14),
          FButton(
            size: FButtonSizeVariant.sm,
            variant: FButtonVariant.outline,
            onPress: onTap,
            child: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('Open Google Cloud'),
                SizedBox(width: 8),
                Icon(FLucideIcons.externalLink, size: 16),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}
