import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_card.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/vtop/edge/edge_events.dart';

class VtopOtpOverlay extends HookConsumerWidget {
  const VtopOtpOverlay({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(vtopOtpChallengeProvider);
    final notifier = ref.read(vtopOtpChallengeProvider.notifier);
    final otpController = useMemoized(
      () => FOtpController(
        children: const [
          FOtpItem(),
          FOtpItem(),
          FOtpItem(),
          FOtpDivider(),
          FOtpItem(),
          FOtpItem(),
          FOtpItem(),
        ],
      ),
    );
    final otpText = useState('');
    final isOtpValid = RegExp(r'^\d{6}$').hasMatch(otpText.value);

    useEffect(() => otpController.dispose, [otpController]);

    useEffect(() {
      if (!state.isActive) {
        otpController.clear();
      }
      return null;
    }, [state.isActive]);

    useEffect(() {
      void listener() {
        otpText.value = otpController.text;
      }

      otpController.addListener(listener);
      return () => otpController.removeListener(listener);
    }, [otpController]);

    String formatSeconds(int totalSeconds) {
      final minutes = (totalSeconds ~/ 60).toString().padLeft(2, '0');
      final seconds = (totalSeconds % 60).toString().padLeft(2, '0');
      return '$minutes:$seconds';
    }

    return Stack(
      children: [
        child,
        if (state.isActive && !state.isMinimized)
          Positioned.fill(
            child: ColoredBox(
              color: context.theme.colors.foreground.withValues(alpha: 0.35),
              // Scrolls on short screens and with the keyboard up, now that
              // the source panel adds height.
              child: Center(
                child: SingleChildScrollView(
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 420),
                    child: Padding(
                      padding: const EdgeInsets.all(20),
                      child: AppCard(
                        title: Row(
                          children: [
                            Expanded(
                              child: Text(
                                'VTOP OTP Verification',
                                style: context.theme.typography.body.md
                                    .copyWith(fontWeight: FontWeight.w700),
                              ),
                            ),
                            FButton.icon(
                              onPress: notifier.minimize,
                              child: const Icon(FLucideIcons.minimize2),
                            ),
                          ],
                        ),
                        subtitle: Text(
                          state.message.isNotEmpty
                              ? state.message
                              : 'Enter the OTP sent to your registered email.',
                        ),
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          spacing: 8,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Align(
                              alignment: Alignment.center,
                              child: FOtpField(
                                control: FOtpFieldControl.managed(
                                  controller: otpController,
                                ),
                                keyboardType: TextInputType.number,
                                label: const Text('OTP'),
                              ),
                            ),
                            if (state.progress case final progress?)
                              _OtpSourcePanel(
                                progress: progress,
                                fetching: state.isAutoFetchingEmail,
                              )
                            else if (state.isAutoFetchingEmail)
                              Text(
                                state.autoFetchMessage ??
                                    'We are trying to get OTP from email...',
                                style: context.theme.typography.body.sm
                                    .copyWith(
                                      color:
                                          context.theme.colors.mutedForeground,
                                    ),
                              ),
                            if (state.canRetryEmailAutoFetch)
                              FButton(
                                variant: FButtonVariant.outline,
                                onPress:
                                    state.remainingSeconds > 0 &&
                                        !state.isSubmitting &&
                                        !state.isResending
                                    ? notifier.retryEmailAutoFetch
                                    : null,
                                child: const Text('Try again'),
                              ),
                            if (state.errorMessage != null &&
                                state.errorMessage!.trim().isNotEmpty)
                              Text(
                                state.errorMessage!,
                                style: context.theme.typography.body.sm
                                    .copyWith(
                                      color: context.theme.colors.destructive,
                                    ),
                              ),

                            Column(
                              crossAxisAlignment: CrossAxisAlignment.center,
                              spacing: 8,
                              children: [
                                Row(
                                  children: [
                                    FButton(
                                      variant: FButtonVariant.outline,
                                      onPress:
                                          state.remainingSeconds == 0 &&
                                              !state.isSubmitting &&
                                              !state.isResending
                                          ? notifier.resendOtp
                                          : null,
                                      child: state.isResending
                                          ? const FCircularProgress()
                                          : const Text('Resend OTP'),
                                    ),
                                    const SizedBox(width: 8),
                                    Flexible(
                                      child: Text(
                                        state.remainingSeconds > 0
                                            ? 'available in ${formatSeconds(state.remainingSeconds)}'
                                            : 'resend available now',
                                        style: context.theme.typography.body.sm
                                            .copyWith(
                                              color: context
                                                  .theme
                                                  .colors
                                                  .mutedForeground,
                                            ),
                                      ),
                                    ),
                                  ],
                                ),
                                const SizedBox(width: 8),
                                FButton(
                                  onPress: state.isSubmitting || !isOtpValid
                                      ? null
                                      : () => notifier.submitOtp(
                                          otpController.text,
                                        ),
                                  child: state.isSubmitting
                                      ? const FCircularProgress()
                                      : const Text('Verify OTP'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        if (state.isActive && state.isMinimized)
          Positioned(
            right: 16,
            bottom: 80,
            child: FTappable(
              onPress: notifier.reopen,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: context.theme.colors.primary,
                  borderRadius: BorderRadius.circular(22),
                  boxShadow: [
                    BoxShadow(
                      blurRadius: 10,
                      color: context.theme.colors.foreground.withValues(
                        alpha: 0.25,
                      ),
                      offset: const Offset(0, 4),
                    ),
                  ],
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 10,
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        state.isAutoFetchingEmail
                            ? FLucideIcons.mailSearch
                            : FLucideIcons.shieldCheck,
                        color: context.theme.colors.primaryForeground,
                        size: 18,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        state.remainingSeconds > 0
                            ? 'OTP ${formatSeconds(state.remainingSeconds)}'
                            : 'OTP',
                        style: context.theme.typography.body.md.copyWith(
                          color: context.theme.colors.primaryForeground,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

String _ago(Duration elapsed) {
  final seconds = elapsed.inSeconds;
  if (seconds < 2) return 'just now';
  if (seconds < 60) return '${seconds}s ago';
  return '${seconds ~/ 60}m ${seconds % 60}s ago';
}

String _clock(Duration duration) {
  final seconds = duration.inSeconds.clamp(0, 5999);
  return '${seconds ~/ 60}:${(seconds % 60).toString().padLeft(2, '0')}';
}

/// How the OTP is being read, step by step, for the method in use. Rebuilt
/// every second with the prompt's countdown, so the times stay current.
class _OtpSourcePanel extends HookWidget {
  const _OtpSourcePanel({required this.progress, required this.fetching});

  final OtpFetchProgress progress;

  /// Still waiting for the mail; false once it stopped (code typed, failed
  /// or expired).
  final bool fetching;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final live = useValueListenable(EdgeEventChannel.instance.connected);
    final now = DateTime.now().toUtc();
    final sinceRequest = now.difference(progress.requestedAt);
    final lastCheck = progress.lastCheckAt == null
        ? null
        : _ago(now.difference(progress.lastCheckAt!));

    final (IconData icon, String title) = switch (progress.source) {
      OtpSource.forwarding => (FLucideIcons.forward, 'Forwarded email'),
      OtpSource.gmail => (FLucideIcons.mail, 'Gmail'),
      OtpSource.manual => (FLucideIcons.keyboard, 'Enter the code yourself'),
    };

    final steps = <_Step>[
      _Step.done('OTP requested from VTOP ${_ago(sinceRequest)}'),
      ...switch (progress.source) {
        OtpSource.manual => [
          const _Step.info(
            'Open the email from VTOP and type the 6-digit code. To fill it '
            'in automatically, set up Email OTP in Settings.',
          ),
        ],
        OtpSource.forwarding => [
          if (fetching)
            const _Step.busy('Waiting for Gmail to forward the OTP email')
          else
            const _Step.info('Stopped waiting for the forwarded email'),
          if (fetching)
            live
                ? const _Step.done('Live connection: it arrives instantly')
                : _Step.info(
                    lastCheck == null
                        ? 'No live connection; checking every 2 s'
                        : 'No live connection; checked ${progress.checks}× · '
                              'last $lastCheck',
                  ),
          if (fetching && progress.gmailFallbackAvailable)
            progress.gmailFallback
                ? const _Step.busy('Reading Gmail too')
                : _Step.info(
                    'Reads Gmail too in '
                    '${_clock(OtpFetchProgress.gmailFallbackAfter - sinceRequest)}',
                  ),
        ],
        OtpSource.gmail => [
          if (fetching)
            _Step.busy(
              lastCheck == null
                  ? 'Checking your inbox'
                  : 'Checking your inbox · ${progress.checks}× · '
                        'last $lastCheck',
            )
          else
            const _Step.info('Stopped checking Gmail'),
          if (progress.deleteAfterReading)
            const _Step.info('The OTP email goes to Trash once read'),
        ],
      },
      if (progress.problem case final problem?) _Step.warning(problem),
    ];

    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.secondary,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 6,
          children: [
            Row(
              spacing: 8,
              children: [
                Icon(icon, size: 16, color: colors.foreground),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        style: typography.body.sm.copyWith(
                          fontWeight: FontWeight.w600,
                          color: colors.foreground,
                        ),
                      ),
                      if (progress.account case final account?)
                        Text(
                          account,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            for (final step in steps) step,
          ],
        ),
      ),
    );
  }
}

enum _StepKind { done, busy, info, warning }

class _Step extends StatelessWidget {
  const _Step.done(this.text) : kind = _StepKind.done;
  const _Step.busy(this.text) : kind = _StepKind.busy;
  const _Step.info(this.text) : kind = _StepKind.info;
  const _Step.warning(this.text) : kind = _StepKind.warning;

  final String text;
  final _StepKind kind;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final color = kind == _StepKind.warning
        ? colors.destructive
        : colors.mutedForeground;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 8,
      children: [
        SizedBox(
          width: 16,
          height: 16,
          child: switch (kind) {
            _StepKind.done => Icon(
              FLucideIcons.check,
              size: 14,
              color: colors.foreground,
            ),
            // The round loader at its own size: squeezed into the box, the
            // icon spins off-centre and wobbles.
            _StepKind.busy => const Center(
              child: FCircularProgress(size: FCircularProgressSizeVariant.xs),
            ),
            _StepKind.info => Icon(FLucideIcons.dot, size: 14, color: color),
            _StepKind.warning => Icon(
              FLucideIcons.triangleAlert,
              size: 14,
              color: color,
            ),
          },
        ),
        Expanded(
          child: Text(
            text,
            style: context.theme.typography.body.xs.copyWith(color: color),
          ),
        ),
      ],
    );
  }
}
