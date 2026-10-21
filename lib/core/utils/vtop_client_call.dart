import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// A VTOP failure in words for the student.
String vtopErrorMessage(Object error) {
  if (error is VtopError) {
    return error.maybeWhen(
      networkError: () => "Couldn't reach VTOP. Check your connection.",
      configurationError: (message) => message,
      vtopServerError: (message) => message.isEmpty
          ? 'VTOP is not responding properly. Try again in a bit.'
          : message,
      sessionExpired: () => 'Your VTOP session expired. Try again.',
      orElse: () => 'Something went wrong. Check the list before retrying.',
    );
  }
  return 'Something went wrong. Check the list before retrying.';
}
