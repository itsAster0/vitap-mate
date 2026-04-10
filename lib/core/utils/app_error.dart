import 'package:http/http.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

String appErrorMessage(Object error) {
  if (error == VtopError.invalidCredentials()) {
    return 'The VTOP username or password is incorrect. Update your password and try again.';
  }
  if (error == VtopError.networkError() || error is ClientException) {
    return "You're offline. Check your connection and try again.";
  }
  if (error == VtopError.sessionExpired()) {
    return 'Your VTOP session expired. Refresh to sign in again.';
  }
  if (error == VtopError.vtopServerError()) {
    return 'VTOP is not responding properly right now. Try again in a bit.';
  }
  if (error == VtopError.captchaRequired()) {
    return 'VTOP needs captcha verification. Please try again.';
  }
  if (error is VtopError_AuthenticationFailed) {
    return 'VTOP authentication failed. Check your details and try again.';
  }
  if (error is VtopError_ParseError || error == VtopError.invalidResponse()) {
    return 'VTOP returned data in an unexpected format. Try refreshing.';
  }
  if (error is FormatException) {
    return error.message;
  }
  if (error is StateError) {
    return error.message;
  }
  if (error is FeatureDisabledException) {
    return 'This feature is currently disabled. Please try again later.';
  }
  if (error is GoogleReauthRequiredException) {
    return error.message;
  }
  return 'Something went wrong. Please try again.';
}
