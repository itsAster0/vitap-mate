import 'package:http/http.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/vtop/vtop_client_call.dart';

enum AppErrorType {
  invalidCredentials,
  network,
  vtopDown,
  sessionExpired,
  server,
  captcha,
  authenticationFailed,
  parse,
  format,
  state,
  unknown,
}

const _offline = "You're offline. Check your connection and try again.";

(AppErrorType, String) appError(Object error) => switch (error) {
  VtopError_InvalidCredentials() => (
    AppErrorType.invalidCredentials,
    'Your VTOP username or password looks incorrect. Check it and try again.',
  ),
  _ when isVtopDown(error) => (AppErrorType.vtopDown, vtopErrorMessage(error)),
  VtopError_NetworkError() ||
  ClientException() => (AppErrorType.network, _offline),
  // DNS / low-level network
  _
      when '$error'.contains('dns error') ||
          '$error'.contains('failed to lookup address') =>
    (AppErrorType.network, _offline),
  VtopError_SessionExpired() => (
    AppErrorType.sessionExpired,
    'Your saved session expired. Please sign in again.',
  ),
  VtopError_VtopServerError(:final field0) => (
    AppErrorType.server,
    'VTOP is not responding properly right now. $field0',
  ),
  VtopError_CaptchaRequired() => (
    AppErrorType.captcha,
    'VTOP needs captcha verification. Please try again.',
  ),
  VtopError_AuthenticationFailed(:final field0) => (
    AppErrorType.authenticationFailed,
    field0.trim().isEmpty
        ? 'We could not complete your VTOP sign-in. Please try again.'
        : field0.trim(),
  ),
  VtopError_ParseError() || VtopError_InvalidResponse() => (
    AppErrorType.parse,
    'VTOP returned data in an unexpected format. Try refreshing.',
  ),
  FormatException(:final message) => (AppErrorType.format, message),
  StateError(:final message) => (AppErrorType.state, message),
  _ => _unknown(error),
};

(AppErrorType, String) _unknown(Object error) {
  Log.info('vtop', 'Unhandled error: $error');
  return (AppErrorType.unknown, 'Something went wrong. Please try again.');
}
