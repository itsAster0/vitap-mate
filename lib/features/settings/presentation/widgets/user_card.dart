import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:local_auth/local_auth.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/features/settings/presentation/pages/user_management.dart';

class UserCard extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserCard({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
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

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: FCard(
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            spacing: 12,
            children: [
              Row(
                spacing: 12,
                children: [
                  DecoratedBox(
                    decoration: BoxDecoration(
                      color: context.theme.colors.primary.withValues(
                        alpha: 0.1,
                      ),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: const Padding(
                      padding: EdgeInsets.all(10),
                      child: Icon(FIcons.idCard),
                    ),
                  ),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          "VTOP Credential",
                          style: context.theme.typography.md.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        Text(
                          user.isValid
                              ? "Account is connected"
                              : "Password needs attention",
                          style: context.theme.typography.sm.copyWith(
                            color: user.isValid
                                ? context.theme.colors.mutedForeground
                                : context.theme.colors.destructive,
                          ),
                        ),
                      ],
                    ),
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
              Wrap(
                alignment: WrapAlignment.center,
                spacing: 10,
                runSpacing: 10,
                children: [
                  UserSemChange(user: user),
                  if (!user.isValid) UserPassChange(user: user),
                  FButton(
                    variant: FButtonVariant.destructive,
                    onPress: () {
                      showAdaptiveDialog(
                        context: context,
                        builder: (dialogContext) => FDialog(
                          title: const Text("Sign out"),
                          body: const Text(
                            "You will be signed out and returned to onboarding.",
                          ),
                          actions: [
                            FButton(
                              variant: FButtonVariant.outline,
                              onPress: () => Navigator.of(dialogContext).pop(),
                              child: const Text("Cancel"),
                            ),
                            FButton(
                              variant: FButtonVariant.destructive,
                              onPress: () async {
                                try {
                                  final username = user.username;
                                  if (username != null && username.isNotEmpty) {
                                    await clearStoredVtopSession(username);
                                    await ref
                                        .read(vtopusersutilsProvider.notifier)
                                        .vtopUserDelete(username);
                                  }
                                  ref.invalidate(vtopUserProvider);
                                  ref.invalidate(vClientProvider);
                                  if (dialogContext.mounted) {
                                    Navigator.of(dialogContext).pop();
                                  }
                                  if (context.mounted) {
                                    context.go('/onboarding');
                                  }
                                } catch (e) {
                                  if (context.mounted) {
                                    disCommonToast(context, e);
                                  }
                                }
                              },
                              child: const Text("Sign out"),
                            ),
                          ],
                        ),
                      );
                    },
                    child: const Text("Sign out"),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
