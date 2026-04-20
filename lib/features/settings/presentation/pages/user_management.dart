import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_login_with_otp.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/settings/presentation/widgets/user_card.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class UserBox extends HookConsumerWidget {
  const UserBox({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final autoRefresh = ref.watch(autoRefreshProvider);

    useEffect(() {
      if (!autoRefresh) return null;

      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          await ref.read(semesterIdProvider.notifier).updatesemids();
          ref.invalidate(semesterIdProvider);
        } catch (e, st) {
          log('auto refresh failed: $e', stackTrace: st);
        }
      });

      return null;
    }, [autoRefresh]);

    final user = ref.watch(vtopUserProvider);

    return user.when(
      data: (data) {
        if ((data.username ?? '').isEmpty) {
          return const SizedBox.shrink();
        }
        return UserCard(user: data);
      },
      error: (e, _) {
        return Center(
          child: Text(
            'Unable to load account details',
            style: TextStyle(color: context.theme.colors.destructive),
          ),
        );
      },
      loading: () {
        return Center(
          child: SizedBox(
            width: 100,
            height: 100,
            child: CircularProgressIndicator(
              color: context.theme.colors.primary,
            ),
          ),
        );
      },
    );
  }
}

class UserSemChange extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserSemChange({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        FButton(
          onPress: () => showAdaptiveDialog(
            context: context,
            builder: (_) => SemesterDialog(user: user, outerContext: context),
          ),
          child: const Text("Change Semester"),
        ),
      ],
    );
  }
}

class SemesterDialog extends HookConsumerWidget {
  final VtopUserEntity user;
  final BuildContext outerContext;

  const SemesterDialog({
    super.key,
    required this.user,
    required this.outerContext,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = useMemoized(
      () => FMultiValueNotifier<String>.radio(user.semid),
    );
    useEffect(() => controller.dispose, [controller]);

    final isRefreshing = useState(false);
    final semester = ref.watch(semesterIdProvider);

    Future<void> handleRefresh() async {
      if (isRefreshing.value) return;

      isRefreshing.value = true;
      try {
        await ref.read(semesterIdProvider.notifier).updatesemids();
        ref.invalidate(semesterIdProvider);
      } catch (e) {
        if (context.mounted) {
          Navigator.of(context).pop();
        }
        if (outerContext.mounted) {
          disCommonToast(outerContext, e);
        }
      } finally {
        isRefreshing.value = false;
      }
    }

    Future<void> handleSemesterChange(Set<String> values) async {
      final selected = values.isEmpty ? null : values.first;
      if (selected == null) return;

      await ref
          .read(vtopusersutilsProvider.notifier)
          .vtopUserSave(user.copyWith(semid: selected));

      ref.invalidate(vtopUserProvider);
      ref.invalidate(vClientProvider);

      final notifSetting = ref.read(classReminderSettingsProvider);
      if (notifSetting.enabled) {
        await ref.read(timetableProvider.notifier).updateTimetable();
      }

      final examNotifSetting = ref.read(examReminderSettingsProvider);
      if (examNotifSetting.enabled) {
        await ref.read(examScheduleProvider.notifier).updatexamschedule();
      }

      if (context.mounted) {
        Navigator.of(context).pop();
      }
    }

    return semester.when(
      data: (data) {
        return FDialog(
          title: Row(
            mainAxisSize: MainAxisSize.max,
            children: [
              const SizedBox(width: 10),
              const Expanded(child: Text('Semesters')),
              if (!isRefreshing.value)
                FTappable(
                  onPress: handleRefresh,
                  child: const Icon(FIcons.rotateCcw),
                )
              else
                const FCircularProgress.pinwheel(),
            ],
          ),
          body: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 300),
            child: SingleChildScrollView(
              child: Column(
                children: [
                  FSelectTileGroup(
                    control: FMultiValueControl.managedRadio(
                      controller: controller,
                      onChange: handleSemesterChange,
                    ),

                    description: const Text('Select the Semester.'),
                    validator: (values) => values?.isEmpty ?? true
                        ? 'Please select a value.'
                        : null,
                    children: [
                      for (final i in data.semesters)
                        FSelectTile(
                          title: Text(i.name, maxLines: 2),
                          value: i.id,
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(context).pop(),
              child: const Text('Cancel'),
            ),
          ],
        );
      },
      error: (e, _) => FDialog(
        title: const Text('Semesters'),
        body: Container(
          decoration: BoxDecoration(color: context.theme.colors.secondary),
          child: Text('$e'),
        ),
        actions: [
          FButton(
            variant: FButtonVariant.outline,
            onPress: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
        ],
      ),
      loading: () => FDialog(
        title: const Text('Semesters'),
        body: Container(
          decoration: BoxDecoration(color: context.theme.colors.secondary),
          child: const FCircularProgress(),
        ),
        actions: [
          FButton(
            variant: FButtonVariant.outline,
            onPress: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
        ],
      ),
    );
  }
}

class UserPassChange extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserPassChange({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final outerContext = context;
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        FButton(
          onPress: () {
            showAdaptiveDialog(
              context: outerContext,
              builder: (dialogContext) {
                return HookBuilder(
                  builder: (context) {
                    final usernameController = useTextEditingController(
                      text: user.username ?? '',
                    );
                    final passwordController = useTextEditingController(
                      text: user.password ?? '',
                    );
                    final isLoading = useState(false);

                    return FDialog(
                      title: const Text('Update Credentials'),
                      body: Column(
                        mainAxisSize: MainAxisSize.min,
                        spacing: 12,
                        children: [
                          FTextField(
                            control: FTextFieldControl.managed(
                              controller: usernameController,
                            ),
                            label: const Text("Username"),
                          ),
                          FTextField.password(
                            control: FTextFieldControl.managed(
                              controller: passwordController,
                            ),
                            obscuringCharacter: '*',
                            label: const Text("New Password"),
                          ),
                        ],
                      ),
                      actions: [
                        if (!isLoading.value)
                          FButton(
                            onPress: () async {
                              final newUsername = usernameController.text
                                  .trim();
                              final newPassword = passwordController.text
                                  .trim();
                              if (newUsername.isEmpty || newPassword.isEmpty) {
                                return;
                              }

                              isLoading.value = true;
                              try {
                                final client = getVtopClient(
                                  username: newUsername,
                                  password: newPassword,
                                );
                                await loginWithSecurityOtpPrompt(
                                  context: context,
                                  client: client,
                                );
                                await ref
                                    .read(vtopusersutilsProvider.notifier)
                                    .vtopUserSave(
                                      user.copyWith(
                                        username: newUsername,
                                        password: newPassword,
                                        isValid: true,
                                      ),
                                    );
                                if (user.username != null &&
                                    user.username != newUsername) {
                                  await clearStoredVtopSession(user.username!);
                                }
                                ref.invalidate(vtopUserProvider);
                                ref.invalidate(vClientProvider);
                              } catch (e) {
                                if (outerContext.mounted) {
                                  disOnbardingCommonToast(outerContext, e);
                                }
                              } finally {
                                isLoading.value = false;
                              }

                              if (dialogContext.mounted) {
                                Navigator.of(dialogContext).pop();
                              }
                            },
                            child: const Text('Update'),
                          )
                        else
                          SizedBox(
                            height: 20,
                            width: 20,
                            child: CircularProgressIndicator(
                              strokeWidth: 2,
                              color: context.theme.colors.primary,
                            ),
                          ),
                        FButton(
                          variant: FButtonVariant.outline,
                          onPress: () => Navigator.of(dialogContext).pop(),
                          child: const Text('Cancel'),
                        ),
                      ],
                    );
                  },
                );
              },
            );
          },
          child: const Text("Update Password"),
        ),
      ],
    );
  }
}
