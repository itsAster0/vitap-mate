import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/vtop_otp_challenge_provider.dart';

class VtopOtpOverlay extends HookConsumerWidget {
  const VtopOtpOverlay({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(vtopOtpChallengeProvider);
    final notifier = ref.read(vtopOtpChallengeProvider.notifier);
    final otpController = useTextEditingController();
    final otpText = useState('');
    final isOtpValid = otpText.value.replaceAll(RegExp(r'\D'), '').length == 6;

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
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 420),
                  child: Padding(
                    padding: const EdgeInsets.all(20),
                    child: FCard(
                      title: Row(
                        children: [
                          Expanded(
                            child: Text(
                              'VTOP OTP Verification',
                              style: context.theme.typography.md.copyWith(
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                          ),
                          FButton.icon(
                            onPress: notifier.minimize,
                            child: const Icon(FIcons.minimize2),
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
                          FTextField(
                            control: FTextFieldControl.managed(
                              controller: otpController,
                            ),
                            keyboardType: TextInputType.number,
                            hint: '6-digit OTP',
                            label: const Text('OTP'),
                          ),
                          if (state.errorMessage != null &&
                              state.errorMessage!.trim().isNotEmpty)
                            Text(
                              state.errorMessage!,
                              style: context.theme.typography.sm.copyWith(
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
                                        ? const FCircularProgress.pinwheel()
                                        : const Text('Resend OTP'),
                                  ),
                                  const SizedBox(width: 8),
                                  Text(
                                    state.remainingSeconds > 0
                                        ? 'available in ${formatSeconds(state.remainingSeconds)}'
                                        : 'resend available now',
                                    style: context.theme.typography.sm.copyWith(
                                      color:
                                          context.theme.colors.mutedForeground,
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
                                    ? const FCircularProgress.pinwheel()
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
                        FIcons.shieldCheck,
                        color: context.theme.colors.primaryForeground,
                        size: 18,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        state.remainingSeconds > 0
                            ? 'OTP ${formatSeconds(state.remainingSeconds)}'
                            : 'OTP',
                        style: context.theme.typography.md.copyWith(
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
