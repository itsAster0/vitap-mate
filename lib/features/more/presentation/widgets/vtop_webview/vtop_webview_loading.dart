import 'dart:async';

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';

class VtopWebviewLoading extends StatefulWidget {
  const VtopWebviewLoading({
    this.error,
    this.onRetry,
    this.reconnecting = false,
    super.key,
  });

  final bool reconnecting;
  final Object? error;
  final VoidCallback? onRetry;

  @override
  State<VtopWebviewLoading> createState() => _VtopWebviewLoadingState();
}

class _VtopWebviewLoadingState extends State<VtopWebviewLoading> {
  Timer? _timer;
  bool _slow = false;

  @override
  void initState() {
    super.initState();
    _startTimer();
  }

  void _startTimer() {
    _timer?.cancel();
    _slow = false;
    if (widget.error == null) {
      _timer = Timer(const Duration(seconds: 15), () {
        if (mounted) setState(() => _slow = true);
      });
    }
  }

  @override
  void didUpdateWidget(covariant VtopWebviewLoading oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.error != widget.error ||
        oldWidget.reconnecting != widget.reconnecting) {
      _startTimer();
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final hasError = widget.error != null;
    final colors = context.theme.colors;
    final typography = context.theme.typography;

    return Container(
      width: double.infinity,
      color: colors.background,
      padding: const EdgeInsets.all(24),
      child: Center(
        child: SingleChildScrollView(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 340),
            child: Column(
              mainAxisSize: MainAxisSize.min,
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
                              FLucideIcons.circleAlert,
                              color: colors.destructive,
                              size: 30,
                            )
                          : const FCircularProgress(
                              semanticsLabel: 'Preparing VTOP session',
                            ),
                    ),
                  ),
                ),
                const SizedBox(height: 20),
                Text(
                  hasError
                      ? 'Could not open VTOP'
                      : widget.reconnecting
                      ? 'Reconnecting to VTOP'
                      : 'Opening VTOP',
                  textAlign: TextAlign.center,
                  style: typography.body.lg.copyWith(
                    color: colors.foreground,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  hasError
                      ? 'Something went wrong while preparing your session.'
                      : _slow
                      ? 'VTOP is taking longer than usual. Please keep waiting.'
                      : 'Preparing your login session securely.',
                  textAlign: TextAlign.center,
                  style: typography.body.sm.copyWith(
                    color: colors.mutedForeground,
                    height: 1.35,
                  ),
                ),
                if (hasError && widget.onRetry != null) ...[
                  const SizedBox(height: 20),
                  FButton(onPress: widget.onRetry, child: const Text('Retry')),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
