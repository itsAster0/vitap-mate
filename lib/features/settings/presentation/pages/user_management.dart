import 'package:app_settings/app_settings.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:forui_hooks/forui_hooks.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:local_auth/local_auth.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

var isLoadingSems = StateProvider<bool>((k) => false);

class UserManagementPage extends HookConsumerWidget {
  const UserManagementPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      color: context.theme.colors.background,
      child: const Padding(padding: EdgeInsets.all(8.0), child: UserBox()),
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
          ref.read(isLoadingSems.notifier).state = true;
          await ref.read(semesterIdProvider.notifier).updatesemids();
          ref.invalidate(semesterIdProvider);
        } catch (_) {
          ();
        } finally {
          ref.read(isLoadingSems.notifier).state = false;
        }
      });
      return null;
    }, []);

    final users = ref.watch(allUsersProviderProvider);
    return ConstrainedBox(
      constraints: BoxConstraints(
        minHeight: MediaQuery.of(context).size.height * 0.8,
      ),
      child: users.when(
        data: (data) {
          return SingleChildScrollView(
            child: Column(
              children: [
                for (final i in data.$1)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 10),
                    child: UserContainer(
                      user: i,
                      isDefault: i.username == data.$2,
                    ),
                  ),
                const SizedBox(height: 20),
                const Useradd(),
              ],
            ),
          );
        },
        error: (e, _) {
          return Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                const Text("Something went worng"),
                FButton(
                  onPress: () {
                    AppSettings.openAppSettings(type: AppSettingsType.settings);
                  },
                  child: const Text("Fix it "),
                ),
                const Text("Click the above button and clear the data"),
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
      ),
    );
  }
}

class UserContainer extends HookConsumerWidget {
  final VtopUserEntity user;
  final bool isDefault;

  const UserContainer({super.key, required this.user, required this.isDefault});

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

    Future<void> setActiveUser() async {
      if (user.username == null) return;
      await ref
          .read(vtopusersutilsProvider.notifier)
          .vtopSetDefault(user.username!);
      ref.invalidate(allUsersProviderProvider);
      ref.invalidate(vtopUserProvider);
      ref.invalidate(vClientProvider);
      try {
        await ref.read(vClientProvider.notifier).tryLogin(force: true);
      } catch (_) {}
      try {
        await ref.read(timetableProvider.notifier).updateTimetable();
      } catch (_) {}
      try {
        await ref.read(examScheduleProvider.notifier).updatexamschedule();
      } catch (_) {}
      try {
        await ref.read(marksProvider.notifier).updatemarks();
      } catch (_) {}
      try {
        await ref.read(attendanceProvider.notifier).updateAttendance();
      } catch (_) {}
    }

    Future<void> deleteUser() async {
      if (user.username == null || isDefault) return;
      await ref
          .read(vtopusersutilsProvider.notifier)
          .vtopUserDelete(user.username!);
      ref.invalidate(allUsersProviderProvider);
      ref.invalidate(vtopUserProvider);
      ref.invalidate(vClientProvider);
      final users =
          await ref.read(vtopusersutilsProvider.notifier).getAllUsers();
      if (users.$1.isEmpty && context.mounted) {
        GoRouter.of(context).go('/onboarding');
      }
    }

    return FFocusedOutline(
      focused: isDefault,
      child: Container(
        decoration: BoxDecoration(
          color: context.theme.colors.primaryForeground,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Padding(
          padding: const EdgeInsets.all(8.0),
          child: Column(
            spacing: 8,
            children: [
              Row(
                spacing: 10,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const Icon(FIcons.idCard),
                  Text(
                    isDefault ? "VTOP Credential (Active)" : "VTOP Credential",
                    style: const TextStyle(fontWeight: FontWeight.bold),
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
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  if (isDefault) UserSemChange(user: user),
                  if (!user.isValid) UserPassChange(user: user),
                ],
              ),
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  if (!isDefault)
                    FButton(
                      style: FButtonStyle.outline(),
                      onPress: setActiveUser,
                      child: const Text("Set Active"),
                    ),
                  if (!isDefault)
                    FButton(
                      style: FButtonStyle.destructive(),
                      onPress: () async {
                        await showAdaptiveDialog(
                          context: context,
                          builder: (dialogContext) {
                            return FDialog(
                              title: const Text("Delete Account"),
                              body: Text(
                                "Delete ${user.username ?? "this account"} from this device?",
                              ),
                              actions: [
                                FButton(
                                  style: FButtonStyle.outline(),
                                  onPress:
                                      () => Navigator.of(dialogContext).pop(),
                                  child: const Text('Cancel'),
                                ),
                                FButton(
                                  style: FButtonStyle.destructive(),
                                  onPress: () async {
                                    await deleteUser();
                                    if (dialogContext.mounted) {
                                      Navigator.of(dialogContext).pop();
                                    }
                                  },
                                  child: const Text('Delete'),
                                ),
                              ],
                            );
                          },
                        );
                      },
                      child: const Text("Delete"),
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

class Useradd extends HookConsumerWidget {
  const Useradd({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final parentContext = context;
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        FButton(
          onPress:
              () => showAdaptiveDialog(
                context: parentContext,
                builder: (dialogContext) {
                  return HookConsumer(
                    builder: (context, ref, _) {
                      final username = useTextEditingController();
                      final password = useTextEditingController();
                      final semController =
                          useFRadioSelectMenuTileGroupController<String>();
                      final semData = useState<SemesterData?>(null);
                      final loading = useState(false);

                      Future<void> verifyAndLoadSemesters() async {
                        final uname = username.text.trim();
                        final pass = password.text.trim();
                        if (uname.isEmpty || pass.isEmpty) return;
                        loading.value = true;
                        try {
                          final client = getVtopClient(
                            username: uname,
                            password: pass,
                          );
                          await vtopClientLogin(client: client);
                          semData.value = await fetchSemesters(client: client);
                        } catch (e) {
                          if (context.mounted) {
                            Navigator.of(context).pop();
                          }
                          if (parentContext.mounted) {
                            showAdaptiveDialog(
                              context: parentContext,
                              builder: (errorContext) {
                                return FDialog(
                                  title: const Text("Unable to add user"),
                                  body: Text("$e"),
                                  actions: [
                                    FButton(
                                      style: FButtonStyle.outline(),
                                      onPress:
                                          () =>
                                              Navigator.of(errorContext).pop(),
                                      child: const Text("Close"),
                                    ),
                                  ],
                                );
                              },
                            );
                          }
                        } finally {
                          loading.value = false;
                        }
                      }

                      Future<void> saveUser() async {
                        final uname = username.text.trim();
                        final pass = password.text.trim();
                        if (uname.isEmpty || pass.isEmpty) return;
                        if (semController.value.isEmpty) return;

                        final util = ref.read(vtopusersutilsProvider.notifier);
                        final existing = await util.getAllUsers();
                        final alreadyExists = existing.$1.any(
                          (u) =>
                              (u.username ?? "").toLowerCase() ==
                              uname.toLowerCase(),
                        );
                        if (alreadyExists) {
                          if (context.mounted) {
                            Navigator.of(context).pop();
                          }
                          if (parentContext.mounted) {
                            showAdaptiveDialog(
                              context: parentContext,
                              builder: (dupContext) {
                                return FDialog(
                                  title: const Text("User already exists"),
                                  body: const Text(
                                    "This account is already added. Skipping duplicate.",
                                  ),
                                  actions: [
                                    FButton(
                                      style: FButtonStyle.outline(),
                                      onPress:
                                          () => Navigator.of(dupContext).pop(),
                                      child: const Text("Close"),
                                    ),
                                  ],
                                );
                              },
                            );
                          }
                          return;
                        }

                        final user = VtopUserEntity(
                          username: uname,
                          password: pass,
                          semid: semController.value.first,
                          isValid: true,
                        );

                        await util.vtopUserSave(user);
                        final allUsers = await util.getAllUsers();
                        if (allUsers.$2 == null || allUsers.$2!.isEmpty) {
                          await util.vtopSetDefault(uname);
                        }

                        ref.invalidate(allUsersProviderProvider);
                        ref.invalidate(vtopUserProvider);
                        ref.invalidate(vClientProvider);
                        if (context.mounted) {
                          Navigator.of(context).pop();
                        }
                      }

                      return FDialog(
                        title: const Text('Add User'),
                        body: Container(
                          decoration: BoxDecoration(
                            color: context.theme.colors.primaryForeground,
                          ),
                          child:
                              semData.value == null
                                  ? Column(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      FTextField(
                                        controller: username,
                                        label: const Text("VTOP Username"),
                                      ),
                                      const SizedBox(height: 8),
                                      FTextField.password(
                                        controller: password,
                                        label: const Text("VTOP Password"),
                                      ),
                                    ],
                                  )
                                  : ConstrainedBox(
                                    constraints: const BoxConstraints(
                                      maxHeight: 320,
                                    ),
                                    child: SingleChildScrollView(
                                      child: FSelectTileGroup(
                                        selectController: semController,
                                        label: const Text('Semesters'),
                                        description: const Text(
                                          'Select the semester for this account.',
                                        ),
                                        children: [
                                          for (final sem
                                              in semData.value!.semesters)
                                            FSelectTile(
                                              title: Text(sem.name),
                                              value: sem.id,
                                            ),
                                        ],
                                      ),
                                    ),
                                  ),
                        ),
                        actions: [
                          FButton(
                            style: FButtonStyle.outline(),
                            onPress: () => Navigator.of(context).pop(),
                            child: const Text('Cancel'),
                          ),
                          if (!loading.value)
                            FButton(
                              onPress:
                                  semData.value == null
                                      ? verifyAndLoadSemesters
                                      : saveUser,
                              child: Text(
                                semData.value == null ? 'Verify' : 'Save',
                              ),
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
                        ],
                      );
                    },
                  );
                },
              ),
          child: const Text('Add User'),
        ),
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
                                            isLoadingSems,
                                          );
                                          void handelClick() async {
                                            try {
                                              ref
                                                  .read(isLoadingSems.notifier)
                                                  .state = true;

                                              await ref
                                                  .read(
                                                    semesterIdProvider.notifier,
                                                  )
                                                  .updatesemids();
                                              ref.invalidate(
                                                semesterIdProvider,
                                              );
                                              ref
                                                  .read(isLoadingSems.notifier)
                                                  .state = false;
                                            } catch (e, _) {
                                              if (context.mounted) {
                                                ref
                                                    .read(
                                                      isLoadingSems.notifier,
                                                    )
                                                    .state = false;

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
                                        ref.invalidate(
                                          allUsersProviderProvider,
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
                    final controller = useTextEditingController();
                    final isLoading = useState(false);

                    return FDialog(
                      title: const Text('Update Password'),
                      body: Container(
                        decoration: BoxDecoration(
                          color: context.theme.colors.primaryForeground,
                        ),
                        child: FTextField.password(
                          controller: controller,
                          obscuringCharacter: '*',
                          label: const Text("New Password"),
                        ),
                      ),
                      actions: [
                        if (!isLoading.value)
                          FButton(
                            onPress: () async {
                              final newPassword = controller.text.trim();
                              if (newPassword.isEmpty) return;

                              isLoading.value = true;
                              try {
                                var client = getVtopClient(
                                  username: user.username!,
                                  password: newPassword,
                                );
                                await vtopClientLogin(client: client);
                                await ref
                                    .read(vtopusersutilsProvider.notifier)
                                    .vtopUserSave(
                                      user.copyWith(
                                        password: newPassword,
                                        isValid: true,
                                      ),
                                    );
                                ref.invalidate(allUsersProviderProvider);
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
                          style: FButtonStyle.outline(),
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
