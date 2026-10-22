import 'package:vitapmate/core/utils/edge/edge_events.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// A VTOP failure in words for the student: specific about what went wrong
/// and what to do, never a bare "something went wrong".
/// Whether [error] means VTOP itself is down or too slow, rather than the
/// phone being offline. A network error counts while the edge's event
/// stream is up: the phone is online, so it is VTOP that can't be reached.
bool isVtopDown(Object error) =>
    error is VtopError_VtopUnavailable ||
    (error == const VtopError.networkError() &&
        EdgeEventChannel.instance.connected.value);

String vtopErrorMessage(Object error) {
  if (error is! VtopError) {
    return 'Something unexpected happened. Please try again.';
  }
  return error.maybeWhen(
    networkError: () => isVtopDown(error)
        ? "VTOP can't be reached right now, though your internet is working. "
              'It may be down; try again in a few minutes.'
        : "Couldn't reach VTOP. Check your internet connection and try again.",
    vtopUnavailable: (reason) =>
        '${reason.trim().isEmpty ? 'VTOP is not answering' : reason.trim()}. '
        "This is on VTOP's side, not your connection; try again in a few minutes.",
    vtopServerError: (message) => message.trim().isEmpty
        ? 'VTOP is not responding properly right now. Try again in a bit.'
        : 'VTOP reported a problem: ${message.trim()}',
    authenticationFailed: (message) => message.trim().isEmpty
        ? "VTOP wouldn't sign you in. Try again, or re-enter your credentials in Settings."
        : 'Sign-in failed: ${message.trim()}',
    registrationParsingError: () =>
        "Couldn't read your registration number from VTOP. Sign in again, or try later.",
    invalidCredentials: () =>
        'VTOP rejected your username or password. Update them in Settings and try again.',
    sessionExpired: () => 'Your VTOP session expired — signing you in again.',
    parseError: (detail) =>
        "VTOP's page couldn't be read${_bracket(detail)}. VTOP may be busy or "
        'under maintenance, or the connection dropped mid-page — try again in a minute.',
    configurationError: (message) => message.trim().isEmpty
        ? 'Something is set up wrong. Check your settings and try again.'
        : message.trim(),
    captchaRequired: () =>
        "Couldn't get past VTOP's captcha this time. Try again.",
    invalidResponse: () =>
        'VTOP sent an unexpected response. Give it a moment and try again.',
    otpRequired: (message, _) => message.trim().isEmpty
        ? 'VTOP needs the one-time code it emailed you.'
        : message.trim(),
    orElse: () => 'Something unexpected happened. Please try again.',
  );
}

/// `" (detail)"` when there is a detail, else empty.
String _bracket(String detail) {
  final trimmed = detail.trim();
  return trimmed.isEmpty ? '' : ' ($trimmed)';
}
