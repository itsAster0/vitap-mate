import 'dart:developer';

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

String _authFailedMessage(Object e) {
  if (e is VtopError) {
    return e.maybeWhen(
      authenticationFailed: (message) {
        final trimmed = message.trim();
        return trimmed.isNotEmpty
            ? trimmed
            : 'We could not complete your sign-in. Please try again.';
      },
      otpRequired: (_) {
        return 'We need to verify this sign-in with the OTP sent to your registered email.';
      },
      vtopServerError: (message) {
        final trimmed = message.trim();
        return trimmed.isNotEmpty
            ? 'VTOP server error: $trimmed'
            : 'VTOP is not responding properly right now. Try again in a bit.';
      },
      sessionExpired: () => 'Your saved session expired. Please sign in again.',
      orElse: () => 'We could not complete your sign-in. Please try again.',
    );
  }
  return 'We could not complete your sign-in. Please try again.';
}

void _showAppToast(
  BuildContext context, {
  required String title,
  required String description,
  VoidCallback? onDismiss,
}) {
  showFToast(
    context: context,
    alignment: FToastAlignment.bottomCenter,
    title: Text(title),
    description: Text(description),
    onDismiss: onDismiss,
    suffixBuilder: (context, entry) => IntrinsicHeight(
      child: FButton(onPress: entry.dismiss, child: const Text('Aye')),
    ),
  );
}

void disCommonToast(BuildContext context, Object e) {
  if (!context.mounted) return;
  if (e == VtopError.invalidCredentials()) {
    _showAppToast(
      context,
      title: 'VTOP Sign-In Failed',
      description:
          'VTOP rejected the saved username or password. Check your VTOP account details in Settings.',
      onDismiss: () {
        final router = GoRouter.maybeOf(context);
        if (router != null) {
          router.goNamed(Paths.settings);
        }
      },
    );
  } else if (e == VtopError.networkError()) {
    _showAppToast(
      context,
      title: 'No Internet Connection',
      description:
          "We couldn't reach VTOP. Check your internet connection and try again.",
    );
  } else if (e is VtopError &&
      e.maybeWhen(
        authenticationFailed: (_) => true,
        otpRequired: (_) => true,
        vtopServerError: (_) => true,
        sessionExpired: () => true,
        orElse: () => false,
      )) {
    _showAppToast(
      context,
      title: 'Sign-In Problem',
      description: _authFailedMessage(e),
    );
  } else if (e is FeatureDisabledException) {
    _showAppToast(
      context,
      title: 'Feature Disabled',
      description: 'Please try again in a while Or Update the app',
    );
  } else {
    log('Unexpected error in disCommonToast: $e');
    _showAppToast(
      context,
      title: 'Error occurred',
      description: 'Please try again',
    );
  }
}

void disOnbardingCommonToast(BuildContext context, Object e) {
  if (!context.mounted) return;
  if (e == VtopError.invalidCredentials()) {
    _showAppToast(
      context,
      title: 'Login Failed',
      description: 'The username or password you entered is incorrect.',
    );
  } else if (e == VtopError.networkError()) {
    _showAppToast(
      context,
      title: 'No Internet Connection',
      description:
          "We couldn't reach VTOP. Check your internet connection and try again.",
    );
  } else if (e is VtopError &&
      e.maybeWhen(
        authenticationFailed: (_) => true,
        otpRequired: (_) => true,
        vtopServerError: (_) => true,
        sessionExpired: () => true,
        orElse: () => false,
      )) {
    _showAppToast(
      context,
      title: 'Login Problem',
      description: _authFailedMessage(e),
    );
  } else if (e is FeatureDisabledException) {
    _showAppToast(
      context,
      title: 'Feature Disabled',
      description: 'Please try again in a while',
    );
  } else {
    _showAppToast(
      context,
      title: 'Login Failed',
      description: 'Authentication failed',
    );
  }
}

void dispToast(BuildContext context, String title, String des) {
  if (!context.mounted) return;
  _showAppToast(context, title: title, description: des);
}
