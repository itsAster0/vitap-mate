import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:forui_hooks/forui_hooks.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:local_auth/local_auth.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

part 'user_management.g.dart';

@Riverpod(keepAlive: true)
class IsLoadingSems extends _$IsLoadingSems {
  @override
  bool build() => false;

  void setLoading(bool value) {
    state = value;
  }
}

class UserManagementPage extends HookConsumerWidget {
  const UserManagementPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      color: context.theme.colors.background,
      child: const Padding(
        padding: EdgeInsets.all(8.0),
        child: SingleChildScrollView(child: UserBox()),
      ),
    );
  }
}

class UserBox extends HookConsumerWidget {
  const UserBox({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        try {
          ref.read(isLoadingSemsProvider.notifier).setLoading(true);
          await ref.read(semesterIdProvider.notifier).updatesemids();
          ref.invalidate(semesterIdProvider);
        } catch (_) {
          ();
        } finally {
          ref.read(isLoadingSemsProvider.notifier).setLoading(false);
        }
      });
      return null;
    }, []);

    final user = ref.watch(vtopUserProvider);
    return user.when(
      data: (data) {
        return UserContainer(user: data);
      },
      error: (e, _) {
        return Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Text("Unable to load VTOP details"),
              FButton(
                onPress: () {
                  GoRouter.of(context).go('/onboarding');
                },
                child: const Text("Set up again"),
              ),
            ],
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

class UserContainer extends HookConsumerWidget {
  final VtopUserEntity user;

  const UserContainer({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auth = useMemoized(() => LocalAuthentication());
    final canUseBiometric =
        useFuture(() async {
          final LocalAuthentication localAuth = LocalAuthentication();
          final bool canAuthenticateWithBiometrics =
              await localAuth.canCheckBiometrics;
          final bool canAuthenticate =
              canAuthenticateWithBiometrics ||
              await localAuth.isDeviceSupported();
          if (canAuthenticate) {
            final List<BiometricType> availableBiometrics =
                await localAuth.getAvailableBiometrics();
            return availableBiometrics.isNotEmpty;
          }
          return false;
        }(), initialData: false).data ??
        false;
    final showPasswords = useState(false);

    return Container(
      decoration: BoxDecoration(
        color: context.theme.colors.primaryForeground,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: context.theme.colors.border),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 14,
          children: [
            Row(
              spacing: 12,
              children: [
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: context.theme.colors.primary.withValues(alpha: .10),
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Icon(
                    FIcons.idCard,
                    color: context.theme.colors.primary,
                  ),
                ),
                Text(
                  "VTOP Account",
                  style: context.theme.typography.lg.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ],
            ),
            _DetailRow(
              label: "Username",
              value: user.username ?? "",
              icon: FIcons.user,
            ),
            Row(
              children: [
                const Icon(FIcons.keyRound, size: 18),
                const SizedBox(width: 10),
                const Text(
                  "Password",
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    showPasswords.value ? user.password ?? "" : "**********",
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (canUseBiometric)
                  FButton.icon(
                    onPress: () async {
                      final List<BiometricType> availableBiometrics =
                          await auth.getAvailableBiometrics();
                      if (availableBiometrics.isNotEmpty &&
                          !showPasswords.value) {
                        final bool didAuthenticate = await auth.authenticate(
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
              spacing: 10,
              mainAxisAlignment: MainAxisAlignment.spaceAround,
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

class _DetailRow extends StatelessWidget {
  final String label;
  final String value;
  final IconData icon;

  const _DetailRow({
    required this.label,
    required this.value,
    required this.icon,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, size: 18),
        const SizedBox(width: 10),
        Text(label, style: const TextStyle(fontWeight: FontWeight.bold)),
        const SizedBox(width: 12),
        Expanded(child: Text(value, overflow: TextOverflow.ellipsis)),
      ],
    );
  }
}

class UserSemChange extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserSemChange({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = useFRadioSelectMenuTileGroupController<String>(
      value: user.semid,
    );
    final outercontext = context;

    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        FButton(
          onPress:
              () => showAdaptiveDialog(
                context: context,
                builder: (context) {
                  return Consumer(
                    builder: (context, ref, child) {
                      var semester = ref.watch(semesterIdProvider);

                      return semester.when(
                        data: (data) {
                          return FDialog(
                            body: ConstrainedBox(
                              constraints: const BoxConstraints(maxHeight: 300),
                              child: SingleChildScrollView(
                                child: Column(
                                  children: [
                                    FSelectTileGroup(
                                      selectController: controller,
                                      label: Consumer(
                                        builder: (context, ref, child) {
                                          final isLoading = ref.watch(
                                            isLoadingSemsProvider,
                                          );
                                          void handelClick() async {
                                            try {
                                              ref
                                                  .read(
                                                    isLoadingSemsProvider
                                                        .notifier,
                                                  )
                                                  .setLoading(true);

                                              await ref
                                                  .read(
                                                    semesterIdProvider.notifier,
                                                  )
                                                  .updatesemids();
                                              ref.invalidate(
                                                semesterIdProvider,
                                              );
                                              ref
                                                  .read(
                                                    isLoadingSemsProvider
                                                        .notifier,
                                                  )
                                                  .setLoading(false);
                                            } catch (e, _) {
                                              if (context.mounted) {
                                                ref
                                                    .read(
                                                      isLoadingSemsProvider
                                                          .notifier,
                                                    )
                                                    .setLoading(false);

                                                Navigator.of(context).pop();
                                              }
                                              if (outercontext.mounted) {
                                                disCommonToast(outercontext, e);
                                              }
                                            }
                                          }

                                          return Row(
                                            mainAxisSize: MainAxisSize.max,
                                            children: [
                                              const SizedBox(width: 10),
                                              const Expanded(
                                                child: Text('Semesters'),
                                              ),
                                              if (!isLoading)
                                                FTappable(
                                                  onPress: handelClick,
                                                  child: const Icon(
                                                    FIcons.rotateCcw,
                                                  ),
                                                ),
                                              if (isLoading)
                                                FCircularProgress.pinwheel(),
                                            ],
                                          );
                                        },
                                      ),
                                      description: const Text(
                                        'Select the Semester.',
                                      ),
                                      onSelect: (value) async {
                                        await ref
                                            .read(
                                              vtopusersutilsProvider.notifier,
                                            )
                                            .vtopUserSave(
                                              user.copyWith(semid: value.$1),
                                            );
                                        ref.invalidate(vtopUserProvider);
                                        ref.invalidate(vClientProvider);
                                        final notifSetting = ref.read(
                                          classReminderSettingsProvider,
                                        );
                                        if (notifSetting.enabled) {
                                          await ref
                                              .read(timetableProvider.notifier)
                                              .updateTimetable();
                                        }
                                        final examNotifSetting = ref.read(
                                          examReminderSettingsProvider,
                                        );
                                        if (examNotifSetting.enabled) {
                                          await ref
                                              .read(
                                                examScheduleProvider.notifier,
                                              )
                                              .updatexamschedule();
                                        }
                                        if (context.mounted) {
                                          Navigator.of(context).pop();
                                        }
                                      },
                                      validator:
                                          (values) =>
                                              values?.isEmpty ?? true
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
                            title: const Text('Semesters'),
                            actions: [
                              FButton(
                                style: FButtonStyle.outline(),
                                onPress: () => Navigator.of(context).pop(),
                                child: const Text('Cancel'),
                              ),
                            ],
                          );
                        },
                        error:
                            (e, et) => FDialog(
                              body: Container(
                                decoration: BoxDecoration(
                                  color: context.theme.colors.primaryForeground,
                                ),
                                child: Text("$e"),
                              ),
                              actions: [
                                FButton(
                                  style: FButtonStyle.outline(),
                                  onPress: () => Navigator.of(context).pop(),
                                  child: const Text('Cancel'),
                                ),
                              ],
                            ),
                        loading:
                            () => FDialog(
                              body: Container(
                                decoration: BoxDecoration(
                                  color: context.theme.colors.primaryForeground,
                                ),
                                child: FCircularProgress(),
                              ),
                              actions: [
                                FButton(
                                  style: FButtonStyle.outline(),
                                  onPress: () => Navigator.of(context).pop(),
                                  child: const Text('Cancel'),
                                ),
                              ],
                            ),
                      );
                    },
                  );
                },
              ),
          child: const Text("Change Semester"),
        ),
      ],
    );
  }
}

class UserPassChange extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserPassChange({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        FButton(
          onPress: () {
            showAdaptiveDialog(
              context: context,
              builder: (dialogContext) => UserPassChangeDialog(user: user),
            );
          },
          child: const Text("Edit Details"),
        ),
      ],
    );
  }
}

class UserPassChangeDialog extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserPassChangeDialog({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final usernameController = useTextEditingController(text: user.username);
    final passwordController = useTextEditingController(text: user.password);
    final isLoading = useState(false);

    return FDialog(
      title: const Text('Edit Details'),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        spacing: 12,
        children: [
          FTextField(
            controller: usernameController,
            label: const Text("VTOP Username"),
            hint: "registration number",
          ),
          FTextField.password(
            controller: passwordController,
            obscuringCharacter: '*',
            label: const Text("VTOP Password"),
            clearable: (k) => k.text.isNotEmpty,
          ),
        ],
      ),
      actions: [
        if (!isLoading.value)
          FButton(
            onPress: () async {
              final newUsername = usernameController.text.trim();
              final newPassword = passwordController.text.trim();
              if (newUsername.isEmpty || newPassword.isEmpty) return;

              isLoading.value = true;
              try {
                var client = getVtopClient(
                  username: newUsername,
                  password: newPassword,
                );
                await vtopClientLogin(client: client);
                final services = await ref.read(appServicesProvider.future);
                await ref
                    .read(vtopusersutilsProvider.notifier)
                    .vtopUserSave(
                      user.copyWith(
                        username: newUsername,
                        password: newPassword,
                        isValid: true,
                      ),
                    );
                await services.preferenceStore.writeCookie(null);
                services.sessionCoordinator.clearClient();
                ref.invalidate(vtopUserProvider);
                ref.invalidate(vClientProvider);
                ref.invalidate(activeAccountProvider);
                if (context.mounted) {
                  Navigator.of(context).pop();
                }
              } catch (e) {
                if (context.mounted) {
                  disOnbardingCommonToast(context, e);
                }
              } finally {
                isLoading.value = false;
              }
            },
            child: const Text('Save'),
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
          style: FButtonStyle.outline(),
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
