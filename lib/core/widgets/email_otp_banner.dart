import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

final emailOtpBannerDismissedProvider = Provider<bool>(
  (ref) =>
      ref
          .watch(settingsProvider)
          .value
          ?.getBool(emailOtpBannerDismissedSettingKey) ??
      false,
);

/// One-time nudge to set up Gmail OTP autofill. Replaces the reminder that
/// used to sit in every screen's header.
class EmailOtpBanner extends ConsumerWidget {
  const EmailOtpBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final needed = ref.watch(emailOtpSetupNeededProvider).value ?? false;
    final dismissed = ref.watch(emailOtpBannerDismissedProvider);
    final show = needed && !dismissed;
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final tone = colors.app.accentTone;

    Future<void> dismiss() async {
      final prefs = await ref.read(settingsProvider.future);
      await prefs.setBool(emailOtpBannerDismissedSettingKey, true);
      ref.invalidate(settingsProvider);
    }

    return AnimatedSize(
      duration: Motion.medium,
      curve: Curves.easeOutCubic,
      alignment: Alignment.topCenter,
      child: !show
          ? const SizedBox(width: double.infinity)
          : Padding(
              padding: const EdgeInsets.fromLTRB(
                Space.sm,
                Space.sm,
                Space.sm,
                0,
              ),
              child: Surface(
                color: tone.subtle,
                borderColor: tone.base.withValues(alpha: 0.25),
                padding: const EdgeInsets.fromLTRB(
                  Space.md + 2,
                  Space.md,
                  Space.xs,
                  Space.md,
                ),
                child: Row(
                  children: [
                    Icon(FLucideIcons.mailCheck, size: 20, color: tone.base),
                    const SizedBox(width: Space.md),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Auto-fill VTOP OTPs',
                            style: typography.body.sm.copyWith(
                              fontWeight: FontWeight.w600,
                              color: tone.onSubtle,
                            ),
                          ),
                          Text(
                            'Connect Gmail so logins need no typing.',
                            style: typography.body.xs.copyWith(
                              color: tone.onSubtle.withValues(alpha: 0.8),
                            ),
                          ),
                        ],
                      ),
                    ),
                    FButton(
                      variant: FButtonVariant.ghost,
                      size: FButtonSizeVariant.sm,
                      mainAxisSize: MainAxisSize.min,
                      onPress: () =>
                          GoRouter.of(context).goNamed(Paths.gmailOtpSetup),
                      child: const Text('Set up'),
                    ),
                    FButton.icon(
                      variant: FButtonVariant.ghost,
                      size: FButtonSizeVariant.sm,
                      semanticsLabel: 'Dismiss',
                      onPress: dismiss,
                      child: Icon(
                        FLucideIcons.x,
                        color: tone.onSubtle.withValues(alpha: 0.7),
                      ),
                    ),
                  ],
                ),
              ),
            ),
    );
  }
}
