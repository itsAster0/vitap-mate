import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/grade_history/grade_history.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/app/router/paths.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:local_auth/local_auth.dart';
import 'package:vitapmate/vtop/clinet_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';
import 'package:vitapmate/vtop/users/vtop_users_utils.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/features/account/user_management.dart';
import 'package:vitapmate/vtop/semesters.dart';

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
    final semesterName = ref
        .watch(semesterIdProvider)
        .value
        ?.semesters
        .where((semester) => semester.id.trim() == semesterId)
        .firstOrNull
        ?.name
        .trim();
    final studentName =
        ref.watch(gradeHistoryProvider).value?.student.name.trim() ?? '';
    final displayName = studentName.isEmpty
        ? username
        : _titleCase(studentName);
    final initials = displayName
        .split(RegExp(r'\s+'))
        .where((p) => p.isNotEmpty)
        .take(2)
        .map((p) => p[0])
        .join();
    final colors = context.theme.colors;
    final typography = context.theme.typography;

    return Surface(
      child: Column(
        spacing: 12,
        children: [
          Row(
            spacing: 12,
            children: [
              Expanded(
                child: PressScale(
                  semanticsLabel: 'Open profile',
                  onPress: () => context.pushNamed(Paths.profile),
                  child: Row(
                    spacing: 12,
                    children: [
                      Container(
                        width: 48,
                        height: 48,
                        alignment: .center,
                        decoration: BoxDecoration(
                          color: colors.app.accentTone.subtle,
                          shape: .circle,
                        ),
                        child: Text(
                          initials.toUpperCase(),
                          style: typography.body.md.copyWith(
                            fontWeight: .w700,
                            color: colors.app.accentTone.onSubtle,
                          ),
                        ),
                      ),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: .start,
                          children: [
                            Text(
                              displayName,
                              maxLines: 1,
                              overflow: .ellipsis,
                              style: typography.body.md.copyWith(
                                fontWeight: .w600,
                                color: colors.foreground,
                              ),
                            ),
                            Text(
                              username,
                              maxLines: 1,
                              overflow: .ellipsis,
                              style: typography.body.xs.copyWith(
                                color: user.isValid
                                    ? colors.mutedForeground
                                    : colors.destructive,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              Semantics(
                label: 'Sign out',
                button: true,
                child: FButton.icon(
                  variant: .ghost,
                  onPress: () => _confirmSignOut(context, ref),
                  child: Icon(
                    FLucideIcons.logOut,
                    color: colors.mutedForeground,
                  ),
                ),
              ),
            ],
          ),
          if (!user.isValid)
            const FAlert(
              variant: .destructive,
              title: Text('Credentials need attention'),
              subtitle: Text('Update your VTOP password to continue.'),
            ),
          // Which semester the app shows, as a field you can tap to change.
          PressScale(
            scale: 0.98,
            semanticsLabel: 'Semester, ${semesterName ?? 'not chosen'}, change',
            onPress: () => showAdaptiveDialog(
              context: context,
              builder: (_) => SemesterDialog(user: user, outerContext: context),
            ),
            child: Container(
              padding: const .symmetric(
                horizontal: Space.md,
                vertical: Space.sm + 2,
              ),
              decoration: BoxDecoration(
                color: colors.secondary,
                borderRadius: .circular(Radii.md),
                border: Border.all(color: colors.border),
              ),
              child: Row(
                children: [
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: colors.app.accentTone.subtle,
                      borderRadius: .circular(Radii.sm + 2),
                    ),
                    child: Icon(
                      FLucideIcons.calendarRange,
                      size: 16,
                      color: colors.app.accentTone.onSubtle,
                    ),
                  ),
                  const SizedBox(width: Space.md),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: .start,
                      children: [
                        Text(
                          'SEMESTER',
                          style: typography.body.xs.copyWith(
                            fontSize: 10,
                            fontWeight: .w600,
                            letterSpacing: 0.6,
                            color: colors.mutedForeground,
                          ),
                        ),
                        Text(
                          semesterName == null || semesterName.isEmpty
                              ? 'Choose semester'
                              : semesterName,
                          maxLines: 1,
                          overflow: .ellipsis,
                          style: typography.body.md.copyWith(
                            fontWeight: .w600,
                            color: colors.foreground,
                          ),
                        ),
                      ],
                    ),
                  ),
                  Icon(
                    FLucideIcons.chevronsUpDown,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
                ],
              ),
            ),
          ),
          Row(
            spacing: 6,
            children: [
              Expanded(child: UserPassChange(user: user)),
              if (hasEnrolledBiometrics)
                Tooltip(
                  message: 'View saved password',
                  child: Semantics(
                    label: 'View saved password',
                    button: true,
                    child: FButton.icon(
                      variant: .outline,
                      size: .sm,
                      onPress: () => _showPassword(context),
                      child: const Icon(FLucideIcons.eye),
                    ),
                  ),
                ),
            ],
          ),
        ],
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

  void _confirmSignOut(BuildContext context, WidgetRef ref) =>
      showAdaptiveDialog(
        context: context,
        builder: (dialogContext) => AppDialog(
          title: const Text('Sign out?'),
          body: const Text(
            'You will be signed out and returned to onboarding.',
          ),
          actions: [
            FButton(
              variant: .outline,
              onPress: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
            FButton(
              variant: .destructive,
              onPress: () async {
                try {
                  final username = user.username;
                  if (username != null && username.isNotEmpty) {
                    await clearStoredVtopSession(username);
                    await ref
                        .read(vtopusersutilsProvider.notifier)
                        .vtopUserDelete(username);
                  }
                  ref
                    ..invalidate(vtopUserProvider)
                    ..invalidate(vClientProvider);
                  if (dialogContext.mounted) Navigator.of(dialogContext).pop();
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

/// "JANE MARY DOE" → "Jane Mary Doe".
String _titleCase(String value) => value
    .toLowerCase()
    .split(' ')
    .map((w) => w.isEmpty ? w : '${w[0].toUpperCase()}${w.substring(1)}')
    .join(' ');
