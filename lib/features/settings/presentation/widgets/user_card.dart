import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:local_auth/local_auth.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/features/settings/presentation/pages/user_management.dart';

class UserCard extends HookWidget {
  final VtopUserEntity user;

  const UserCard({super.key, required this.user});

  @override
  Widget build(BuildContext context) {
    final auth = useMemoized(() => LocalAuthentication());
    final canUseBiometric =
        useFuture(() async {
          final localAuth = LocalAuthentication();
          final canAuthenticateWithBiometrics =
              await localAuth.canCheckBiometrics;
          final canAuthenticate =
              canAuthenticateWithBiometrics ||
              await localAuth.isDeviceSupported();
          if (canAuthenticate) {
            final availableBiometrics = await localAuth
                .getAvailableBiometrics();
            return availableBiometrics.isNotEmpty;
          }
          return false;
        }(), initialData: false).data ??
        false;
    final showPasswords = useState(false);

    return FCard(
      child: Padding(
        padding: const EdgeInsets.all(8.0),
        child: Column(
          spacing: 8,
          children: [
            const Row(
              spacing: 10,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(FIcons.idCard),
                Text(
                  "VTOP Credential",
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ],
            ),
            Row(
              children: [
                const Text(
                  "Username : ",
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                Expanded(child: Text(user.username ?? "")),
              ],
            ),
            Row(
              children: [
                Expanded(
                  child: Row(
                    children: [
                      const Text(
                        "Password : ",
                        style: TextStyle(fontWeight: FontWeight.bold),
                      ),
                      Text(
                        showPasswords.value
                            ? user.password ?? ""
                            : "**********",
                      ),
                    ],
                  ),
                ),
                if (canUseBiometric)
                  FButton.icon(
                    onPress: () async {
                      final availableBiometrics = await auth
                          .getAvailableBiometrics();
                      if (availableBiometrics.isNotEmpty &&
                          !showPasswords.value) {
                        final didAuthenticate = await auth.authenticate(
                          localizedReason:
                              'Please authenticate to show Password',
                        );
                        if (!didAuthenticate) return;
                      }
                      showPasswords.value = !showPasswords.value;
                    },
                    child: const Icon(FIcons.eye),
                  ),
              ],
            ),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                UserSemChange(user: user),
                if (!user.isValid) UserPassChange(user: user),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
