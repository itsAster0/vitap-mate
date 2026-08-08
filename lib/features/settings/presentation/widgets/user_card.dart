import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
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
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';

class UserCard extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserCard({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final hasEnrolledBiometrics =
        useFuture(
          useMemoized(() async {
            try {
              return (await LocalAuthentication().getAvailableBiometrics())
                  .isNotEmpty;
            } catch (_) {
              return false;
            }
          }, const []),
          initialData: false,
        ).data ??
        false;
    final username = user.username?.trim().toUpperCase() ?? '';
    final semesterId = user.semid?.trim();
    final semesters =
        ref.watch(semesterIdProvider).value?.semesters ?? const [];
    String? semesterName;
    for (final semester in semesters) {
      if (semester.id.trim() == semesterId) {
        semesterName = semester.name.trim();
        break;
      }
    }

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: FCard(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            spacing: 16,
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
                      child: Icon(FLucideIcons.idCard),
                    ),
                  ),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'VTOP Account',
                          style: context.theme.typography.body.md.copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        Text(
                          [
                            username,
                            if (semesterName != null && semesterName.isNotEmpty)
                              semesterName,
                          ].join('  •  '),
                          style: context.theme.typography.body.sm.copyWith(
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
              if (!user.isValid) ...[
                const FAlert(
                  variant: FAlertVariant.destructive,
                  title: Text('Credentials need attention'),
                  subtitle: Text('Update your VTOP password to continue.'),
                ),
              ],
              Row(
                spacing: 6,
                children: [
                  Expanded(child: UserPassChange(user: user)),
                  Semantics(
                    label: 'Sign out',
                    button: true,
                    child: FButton.icon(
                      onPress: () => _confirmSignOut(context, ref),
                      child: const Icon(FLucideIcons.logOut),
                    ),
                  ),
                ],
              ),
              Row(
                spacing: 6,
                children: [
                  Expanded(
                    child: FButton(
                      variant: FButtonVariant.outline,
                      onPress: () => showAdaptiveDialog(
                        context: context,
                        builder: (_) =>
                            SemesterDialog(user: user, outerContext: context),
                      ),
                      child: const Text('Change semester'),
                    ),
                  ),
                  if (hasEnrolledBiometrics) ...[
                    Tooltip(
                      message: 'View saved password',
                      child: Semantics(
                        label: 'View saved password',
                        button: true,
                        child: FButton.icon(
                          onPress: () => _showPassword(context),
                          child: const Icon(FLucideIcons.eye),
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Future<void> _showPassword(BuildContext context) async {
    final password = user.password;
    if (password == null || password.isEmpty) {
      dispToast(context, 'Unavailable', 'No saved password was found.');
      return;
    }

    try {
      final auth = LocalAuthentication();
      if ((await auth.getAvailableBiometrics()).isEmpty) {
        if (context.mounted) {
          dispToast(
            context,
            'Authentication Required',
            'Set up biometric authentication first.',
          );
        }
        return;
      }

      final authenticated = await auth.authenticate(
        localizedReason: 'Authenticate to view your saved VTOP password',
      );
      if (!authenticated || !context.mounted) return;

      await showAdaptiveDialog<void>(
        context: context,
        builder: (dialogContext) => AppDialog(
          title: const Text('Saved VTOP Password'),
          body: SelectableText(password),
          actions: [
            FButton(
              onPress: () => Navigator.of(dialogContext).pop(),
              child: const Text('Done'),
            ),
          ],
        ),
      );
    } catch (error) {
      if (context.mounted) disCommonToast(context, error);
    }
  }

  void _confirmSignOut(BuildContext context, WidgetRef ref) {
    showAdaptiveDialog(
      context: context,
      builder: (dialogContext) => AppDialog(
        title: const Text('Sign out?'),
        body: const Text('You will be signed out and returned to onboarding.'),
        actions: [
          FButton(
            variant: FButtonVariant.outline,
            onPress: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
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
                if (context.mounted) context.go('/onboarding');
              } catch (error) {
                if (context.mounted) disCommonToast(context, error);
              }
            },
            child: const Text('Sign out'),
          ),
        ],
      ),
    );
  }
}
