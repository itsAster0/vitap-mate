import 'dart:developer' show log;

import 'package:http/http.dart';
import 'package:vitapmate/vtop/vtop_client_call.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

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
  discontinued,
  reauthRequired,
  unknown,
}

(AppErrorType, String) appError(Object error) {
  if (error == VtopError.invalidCredentials()) {
    return (
      AppErrorType.invalidCredentials,
      'Your VTOP username or password looks incorrect. Check it and try again.',
    );
  }

  if (isVtopDown(error)) {
    return (AppErrorType.vtopDown, vtopErrorMessage(error));
  }

  if (error == VtopError.networkError() || error is ClientException) {
    return (
      AppErrorType.network,
      "You're offline. Check your connection and try again.",
    );
  }

  // DNS / low-level network
  if (error.toString().contains('dns error') ||
      error.toString().contains('failed to lookup address')) {
    return (
      AppErrorType.network,
      "You're offline. Check your connection and try again.",
    );
  }

  if (error == VtopError.sessionExpired()) {
    return (
      AppErrorType.sessionExpired,
      'Your saved session expired. Please sign in again.',
    );
  }

  if (error is VtopError_VtopServerError) {
    return (
      AppErrorType.server,
      'VTOP is not responding properly right now. ${error.field0}',
    );
  }

  if (error == VtopError.captchaRequired()) {
    return (
      AppErrorType.captcha,
      'VTOP needs captcha verification. Please try again.',
    );
  }

  if (error is VtopError_AuthenticationFailed) {
    final message = error.field0.trim();
    return (
      AppErrorType.authenticationFailed,
      message.isEmpty
          ? 'We could not complete your VTOP sign-in. Please try again.'
          : message,
    );
  }

  if (error is VtopError_ParseError || error == VtopError.invalidResponse()) {
    return (
      AppErrorType.parse,
      'VTOP returned data in an unexpected format. Try refreshing.',
    );
  }

  if (error is FormatException) {
    return (AppErrorType.format, error.message);
  }

  if (error is StateError) {
    return (AppErrorType.state, error.message);
  }

  log('Unhandled error: $error');
  return (AppErrorType.unknown, 'Something went wrong. Please try again.');
}

extension AppErrorTypeX on AppErrorType {
  String get title {
    switch (this) {
      case AppErrorType.invalidCredentials:
        return 'Invalid Credentials';
      case AppErrorType.network:
        return 'No Internet';
      case AppErrorType.vtopDown:
        return 'VTOP Is Down';
      case AppErrorType.sessionExpired:
        return 'Session Expired';
      case AppErrorType.server:
        return 'Server Error';
      case AppErrorType.captcha:
        return 'Captcha Required';
      case AppErrorType.authenticationFailed:
        return 'Authentication Failed';
      case AppErrorType.parse:
        return 'Data Error';
      case AppErrorType.format:
        return 'Format Error';
      case AppErrorType.state:
        return 'State Error';
      case AppErrorType.discontinued:
        return 'Feature Discontinued';
      case AppErrorType.reauthRequired:
        return 'Re-authentication Required';
      case AppErrorType.unknown:
        return 'Error';
    }
  }
}
