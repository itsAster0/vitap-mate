import 'package:flutter/material.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/vtop/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';

bool isSecurityOtpRequiredError(Object error) => error is VtopError_OTPRequired;

DateTime? securityOtpRequiredAt(Object error) => switch (error) {
  VtopError_OTPRequired(:final field1) => utcFromUnixSeconds(field1),
  _ => null,
};

Future<void> loginWithSecurityOtpPrompt({
  required BuildContext context,
  required VtopClient client,
}) async {
  try {
    await ProviderScope.containerOf(
      context,
      listen: false,
    ).read(vtopAuthenticatorProvider).login(client);
  } catch (error) {
    if (!isSecurityOtpRequiredError(error)) rethrow;
    if (!context.mounted) {
      throw VtopError.authenticationFailed('OTP verification cancelled');
    }
    await ProviderScope.containerOf(context, listen: false)
        .read(vtopOtpChallengeProvider.notifier)
        .requestOtp(
          client: client,
          otpRequiredAt: securityOtpRequiredAt(error),
        );
    if (!await fetchIsAuth(client: client)) {
      throw VtopError.authenticationFailed(
        'OTP verification completed but login session is not authenticated.',
      );
    }
  }
}
