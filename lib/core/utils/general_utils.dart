import 'package:intl/intl.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

String formatUnixTimestamp(int timestamp) {
  final date = DateTime.fromMillisecondsSinceEpoch(timestamp * 1000);
  final formatter = DateFormat("MMM dd, yyyy hh:mm a");
  return formatter.format(date).toLowerCase();
}

String commonErrorMessage(Object e) {
  if (e == VtopError.invalidCredentials()) {
    return 'It looks like you changed your VTOP password. Please update it.';
  } else if (e == VtopError.networkError()) {
    return "You're offline. Please check your connection and try again.";
  } else if (e is FeatureDisabledException) {
    return 'This feature is currently disabled. Please try again in a while';
  } else {
    return "Something went wrong. Please try reopening the app.";
  }
}
