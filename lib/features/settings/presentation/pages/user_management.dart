import 'package:app_settings/app_settings.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:forui_hooks/forui_hooks.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class UserManagementPage extends HookConsumerWidget {
  const UserManagementPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Padding(padding: const EdgeInsets.all(8.0), child: UserBox());
  }
}

class UserBox extends HookConsumerWidget {
  const UserBox({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
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
              child: CircularProgressIndicator(color: Colors.black),
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
    return FFocusedOutline(
      focused: isDefault,
      child: FTappable(
        onPress: () {},
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
                    Text(
                      "Password : ",
                      style: TextStyle(fontWeight: FontWeight.bold),
                    ),
                    Text("*" * (10)),
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
          intrinsicWidth: true,
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
                        style: FButtonStyle.outline,
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
    var semester = ref.watch(semesterIdProvider);
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        FButton(
          intrinsicWidth: true,
          onPress:
              () => showAdaptiveDialog(
                context: context,
                builder: (context) {
                  return semester.when(
                    data: (data) {
                      return FDialog(
                        body: ConstrainedBox(
                          constraints: BoxConstraints(maxHeight: 300),
                          child: Container(
                            decoration: BoxDecoration(
                              color: context.theme.colors.primaryForeground,
                            ),
                            child: FSelectTileGroup(
                              selectController: controller,
                              label: const Text('Semesters'),
                              description: const Text('Select the Semester.'),
                              onSelect: (value) async {
                                await ref
                                    .read(vtopusersutilsProvider.notifier)
                                    .vtopUserSave(
                                      user.copyWith(semid: value.$1),
                                    );
                                ref.invalidate(vtopUserProvider);
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
                          ),
                        ),
                        title: const Text('Semesters'),
                        actions: [
                          FButton(
                            style: FButtonStyle.outline,
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
                              style: FButtonStyle.outline,
                              onPress: () => Navigator.of(context).pop(),
                              child: const Text('Cancel'),
                            ),
                          ],
                        ),
                    loading:
                        () => CircularProgressIndicator(color: Colors.black),
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
          intrinsicWidth: true,
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
                          const SizedBox(
                            height: 20,
                            width: 20,
                            child: CircularProgressIndicator(
                              strokeWidth: 2,
                              color: Colors.black,
                            ),
                          ),
                        FButton(
                          style: FButtonStyle.outline,
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
