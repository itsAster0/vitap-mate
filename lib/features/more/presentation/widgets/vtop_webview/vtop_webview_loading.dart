import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';

class VtopWebviewLoading extends StatelessWidget {
  const VtopWebviewLoading({this.error, this.onRetry, super.key});

  final Object? error;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final hasError = error != null;
    final colors = context.theme.colors;
    final typography = context.theme.typography;

    return FScaffold(
      childPad: false,
      header: FHeader.nested(
        title: const Text('VTOP'),
        prefixes: [
          FHeaderAction.back(onPress: () => GoRouter.of(context).pop()),
        ],
      ),
      child: Container(
        width: double.infinity,
        color: colors.background,
        padding: const EdgeInsets.all(24),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 340),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                DecoratedBox(
                  decoration: BoxDecoration(
                    color: colors.primary.withValues(alpha: 0.08),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: hasError
                          ? colors.destructive.withValues(alpha: 0.28)
                          : colors.primary.withValues(alpha: 0.24),
                    ),
                  ),
                  child: SizedBox.square(
                    dimension: 72,
                    child: Center(
                      child: hasError
                          ? Icon(
                              FIcons.circleAlert,
                              color: colors.destructive,
                              size: 30,
                            )
                          : SizedBox.square(
                              dimension: 30,
                              child: CircularProgressIndicator(
                                strokeWidth: 3,
                                color: colors.primary,
                                backgroundColor: colors.border.withValues(
                                  alpha: 0.35,
                                ),
                              ),
                            ),
                    ),
                  ),
                ),
                const SizedBox(height: 20),
                Text(
                  hasError ? 'Could not open VTOP' : 'Opening VTOP',
                  textAlign: TextAlign.center,
                  style: typography.lg.copyWith(
                    color: colors.foreground,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  hasError
                      ? 'Something went wrong while preparing your session.'
                      : 'Preparing your login session securely.',
                  textAlign: TextAlign.center,
                  style: typography.sm.copyWith(
                    color: colors.mutedForeground,
                    height: 1.35,
                  ),
                ),
                if (hasError && onRetry != null) ...[
                  const SizedBox(height: 20),
                  FButton(onPress: onRetry, child: const Text('Retry')),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
