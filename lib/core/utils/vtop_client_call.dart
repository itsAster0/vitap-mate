import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Runs [call] on the device's VTOP client, signing in first if needed and
/// once more if VTOP has dropped the session.
///
/// For live, uncached pages (outings, the course page) that go straight
/// from the phone to VTOP even when a vtop-server is configured.
Future<T> withVtopClient<T>(
  Ref ref,
  Future<T> Function(VtopClient client) call,
) async {
  final login = ref.read(vClientProvider.notifier);
  await login.ensureLogin();
  try {
    return await call(await ref.read(vClientProvider.future));
  } on VtopError_SessionExpired {
    await login.ensureLogin(force: true);
    return call(await ref.read(vClientProvider.future));
  }
}

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
