import 'package:flutter/material.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

bool _isSecurityOtpRequiredError(Object error) {
  if (error is! VtopError) return false;
  return error.maybeWhen(otpRequired: (_) => true, orElse: () => false);
}

Future<void> loginWithSecurityOtpPrompt({
  required BuildContext context,
  required VtopClient client,
}) async {
  try {
    await vtopClientLogin(client: client);
    return;
  } catch (error) {
    if (!_isSecurityOtpRequiredError(error)) rethrow;
  }

  if (!context.mounted) {
    throw VtopError.authenticationFailed('OTP verification cancelled');
  }
  final container = ProviderScope.containerOf(context, listen: false);
  await container
      .read(vtopOtpChallengeProvider.notifier)
      .requestOtp(
        client: client,
        message:
            'Additional verification required. OTP sent to your registered email.',
      );
  if (!await fetchIsAuth(client: client)) {
    throw VtopError.authenticationFailed(
      'OTP verification completed but login session is not authenticated.',
    );
  }
}

bool isSecurityOtpRequiredError(Object error) {
  return _isSecurityOtpRequiredError(error);
}

Future<void> requestSecurityOtpForClient({
  required ProviderContainer container,
  required VtopClient client,
  String message =
      'Additional verification required. OTP sent to your registered email.',
}) {
  return container
      .read(vtopOtpChallengeProvider.notifier)
      .requestOtp(client: client, message: message);
}
