import 'package:app_settings/app_settings.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:forui_hooks/forui_hooks.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:local_auth/local_auth.dart';

var isLoadingSems = StateProvider<bool>((k) => false);

class UserManagementPage extends HookConsumerWidget {
  const UserManagementPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      color: context.theme.colors.background,
      child: Padding(padding: const EdgeInsets.all(8.0), child: UserBox()),
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
        } catch (e, _) {
          ();
        } finally {
          ref.read(isLoadingSems.notifier).state = false;
        }
      });
      return null;
    }, []);

    var users = ref.watch(allUsersProviderProvider);
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
                  UserContainer(user: i, isDefault: i.username == data.$2),
                SizedBox(height: 20),
                Useradd(),
              ],
            ),
          );
        },
        error: (e, se) {
          return Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Text("Something went worng"),
                FButton(
                  onPress: () {
                    AppSettings.openAppSettings(type: AppSettingsType.settings);
                  },
                  child: Text("Fix it "),
                ),
                Text("Click the above button and clear the data"),
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
          final LocalAuthentication auth = LocalAuthentication();
          final bool canAuthenticateWithBiometrics =
              await auth.canCheckBiometrics;
          final bool canAuthenticate =
              canAuthenticateWithBiometrics || await auth.isDeviceSupported();
          if (canAuthenticate) {
            final List<BiometricType> availableBiometrics =
                await auth.getAvailableBiometrics();
            return availableBiometrics.isNotEmpty;
          }
          return false;
        }(), initialData: false).data ??
        false;
    final showPasswords = useState(false);
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
            spacing: 5,
            children: [
              Row(
                spacing: 10,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(FIcons.idCard),
                  Text(
                    "Vtop Credential ",
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                ],
              ),
              SizedBox(height: 5),
              Row(
                children: [
                  Text(
                    "Username : ",
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                  Text(user.username!),
                ],
              ),
              Row(
                children: [
                  Expanded(
                    child: Row(
                      children: [
                        Text(
                          "Password : ",
                          style: TextStyle(fontWeight: FontWeight.bold),
                        ),

                        Text(
                          showPasswords.value
                              ? user.password ?? ""
                              : "*" * (10),
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
                      child: Icon(FIcons.eye),
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
      ),
    );
  }
}

class Useradd extends HookConsumerWidget {
  const Useradd({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    //var mainContext = context;
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        FButton(
          onPress:
              () => showAdaptiveDialog(
                context: context,
                builder: (context) {
                  return FDialog(
                    body: Container(
                      decoration: BoxDecoration(
                        color: context.theme.colors.primaryForeground,
                      ),
                      child: Text("soon"),
                    ),
                    actions: [
                      FButton(
                        style: FButtonStyle.outline(),
                        onPress: () => Navigator.of(context).pop(),
                        child: const Text('Cancel'),
                      ),
                      // FButton(onPress: () => Navigator.of(context).pop(), child: const Text('Continue'))
                    ],
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
                              constraints: BoxConstraints(maxHeight: 300),
                              child: Container(
                                decoration: BoxDecoration(),
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
                                                    .read(
                                                      isLoadingSems.notifier,
                                                    )
                                                    .state = true;

                                                await ref
                                                    .read(
                                                      semesterIdProvider
                                                          .notifier,
                                                    )
                                                    .updatesemids();
                                                ref.invalidate(
                                                  semesterIdProvider,
                                                );
                                                ref
                                                    .read(
                                                      isLoadingSems.notifier,
                                                    )
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
                                                  disCommonToast(
                                                    outercontext,
                                                    e,
                                                  );
                                                }
                                              }
                                            }

                                            return Row(
                                              mainAxisSize: MainAxisSize.max,
                                              children: [
                                                // if (isLoading)
                                                //   const FCircularProgress.pinwheel(),
                                                // if (isLoading)
                                                const SizedBox(width: 10),
                                                Expanded(
                                                  child: Text('Semesters'),
                                                ),
                                                if (!isLoading)
                                                  FTappable(
                                                    onPress: handelClick,
                                                    child: Icon(
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
                                          final notifSetting = ref.read(
                                            classReminderSettingsProvider,
                                          );
                                          if (notifSetting.enabled) {
                                            await ref
                                                .read(
                                                  timetableProvider.notifier,
                                                )
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
                                        //maxHeight: MediaQuery.of(context).size.height * 075,
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
                            ),
                            title: const Text('Semesters'),
                            actions: [
                              FButton(
                                style: FButtonStyle.outline(),
                                onPress: () => Navigator.of(context).pop(),
                                child: const Text('Cancel'),
                              ),
                              // FButton(onPress: () => Navigator.of(context).pop(), child: const Text('Continue'))
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
                                ref.invalidate(vtopUserProvider);
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
