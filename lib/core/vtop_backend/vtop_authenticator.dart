import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

/// Signs a [VtopClient] in on the device. (The edge signs in on its own;
/// see EdgeVtopBackend.)
///
/// It leaves the signed-in session in the local client, so persisting, the
/// WebView cookie bridge and local fetches keep working unchanged. It
/// throws [VtopError.otpRequired] when VTOP wants the emailed code, which the
/// existing OTP prompt then sends to [submitOtp].
abstract interface class VtopAuthenticator {
  Future<void> login(VtopClient client);
  Future<void> submitOtp(VtopClient client, String otp);
  Future<void> resendOtp(VtopClient client);
}

/// Logs in with the bundled Rust library; the password stays on the phone.
class LocalVtopAuthenticator implements VtopAuthenticator {
  const LocalVtopAuthenticator();

  @override
  Future<void> login(VtopClient client) =>
      vtop_api.vtopClientLogin(client: client);

  @override
  Future<void> submitOtp(VtopClient client, String otp) =>
      vtop_api.vtopClientSubmitSecurityOtp(client: client, otpCode: otp);

  @override
  Future<void> resendOtp(VtopClient client) =>
      vtop_api.vtopClientResendSecurityOtp(client: client);
}
