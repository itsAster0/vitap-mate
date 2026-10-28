import 'package:vitapmate/observability/log.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/app/router/paths.dart';
import 'package:vitapmate/vtop/vtop_client_call.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

const _signInFailed = 'We could not complete your sign-in. Please try again.';

String _authFailedMessage(Object e) => switch (e) {
  VtopError() => e.maybeWhen(
    authenticationFailed: (message) =>
        message.trim().isNotEmpty ? message.trim() : _signInFailed,
    otpRequired: (_, _) =>
        'We need to verify this sign-in with the OTP sent to your registered email.',
    vtopServerError: (message) => message.trim().isNotEmpty
        ? 'VTOP server error: ${message.trim()}'
        : 'VTOP is not responding properly right now. Try again in a bit.',
    sessionExpired: () => 'Your saved session expired. Please sign in again.',
    orElse: () => _signInFailed,
  ),
  _ => _signInFailed,
};

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
    // Long errors would push the toast off the screen.
    description: Text(
      description,
      maxLines: 5,
      overflow: TextOverflow.ellipsis,
    ),
    onDismiss: onDismiss,
    suffixBuilder: (context, entry) => IntrinsicHeight(
      child: FButton(onPress: entry.dismiss, child: const Text('Aye')),
    ),
  );
}

const _offline = (
  'No Internet Connection',
  "We couldn't reach VTOP. Check your internet connection and try again.",
);

bool _isSignInError(Object e) =>
    e is VtopError &&
    e.maybeWhen(
      authenticationFailed: (_) => true,
      otpRequired: (_, _) => true,
      vtopServerError: (_) => true,
      sessionExpired: () => true,
      orElse: () => false,
    );

void disCommonToast(BuildContext context, Object e) {
  if (!context.mounted) return;
  final invalidCredentials = e == VtopError.invalidCredentials();
  final (title, description) = switch (e) {
    _ when invalidCredentials => (
      'VTOP Sign-In Failed',
      'VTOP rejected the saved username or password. Check your VTOP account details in Settings.',
    ),
    _ when isVtopDown(e) => ('VTOP Is Down', vtopErrorMessage(e)),
    _ when e == VtopError.networkError() => _offline,
    _ when _isSignInError(e) => ('Sign-In Problem', _authFailedMessage(e)),
    VtopError() => ('VTOP Problem', vtopErrorMessage(e)),
    _ => () {
      Log.info('app.widgets', 'Unexpected error in disCommonToast: $e');
      return ('Error occurred', 'Please try again');
    }(),
  };
  _showAppToast(
    context,
    title: title,
    description: description,
    onDismiss: invalidCredentials
        ? () => GoRouter.maybeOf(context)?.goNamed(Paths.settings)
        : null,
  );
}

void disOnboardingCommonToast(BuildContext context, Object e) {
  if (!context.mounted) return;
  final (title, description) = switch (e) {
    _ when e == VtopError.invalidCredentials() => (
      'Login Failed',
      'The username or password you entered is incorrect.',
    ),
    _ when isVtopDown(e) => ('VTOP Is Down', vtopErrorMessage(e)),
    _ when e == VtopError.networkError() => _offline,
    _ when _isSignInError(e) => ('Login Problem', _authFailedMessage(e)),
    VtopError() => ('Login Problem', vtopErrorMessage(e)),
    _ => ('Login Failed', 'Authentication failed'),
  };
  _showAppToast(context, title: title, description: description);
}

void dispToast(BuildContext context, String title, String des) {
  if (!context.mounted) return;
  _showAppToast(context, title: title, description: des);
}
