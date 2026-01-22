import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';

import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:forui_hooks/forui_hooks.dart';

late VtopClient _globalClient;
late String _globalUsername;
late String _globalPassword;

class OnboardingPage extends HookConsumerWidget {
  const OnboardingPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final stepTwo = useState(false);
    return FScaffold(
      header: FHeader.nested(
        title: Text("Vitap Mate"),
        suffixes: [
          // FButton(
          //   onPress: () {
          //     showDialog(
          //       context: context,
          //       builder: (BuildContext conntext) {
          //         //return Center(child: WifiCard());
          //         return Center(
          //           child: Column(
          //             mainAxisAlignment: MainAxisAlignment.center,
          //             children: [
          //               Padding(
          //                 padding: const EdgeInsets.all(16.0),
          //                 child: WifiCard(),
          //               ),
          //             ],
          //           ),
          //         );
          //       },
          //     );
          //   },
          //   child: Icon(FIcons.wifi),
          // ),
        ],
      ),
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.only(right: 50, left: 50, top: 10),
            child: Row(
              children: [
                CircleAvatar(
                  backgroundColor: Colors.black,
                  minRadius: 25,
                  child: const Text("1", style: TextStyle(color: Colors.white)),
                ),
                Expanded(
                  child: Container(
                    height: 4,
                    color: !stepTwo.value ? Colors.grey : Colors.black,
                  ),
                ),
                CircleAvatar(
                  minRadius: 25,
                  backgroundColor: !stepTwo.value ? Colors.grey : Colors.black,
                  child: Text(
                    "2",
                    style: TextStyle(
                      color: stepTwo.value ? Colors.white : Colors.black,
                    ),
                  ),
                ),
              ],
            ),
          ),
          Expanded(child: !stepTwo.value ? Step1(two: stepTwo) : Step2()),
        ],
      ),
    );
  }
}

class Step1 extends HookConsumerWidget {
  final ValueNotifier two;
  const Step1({super.key, required this.two});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final formKey = useMemoized(() => GlobalKey<FormState>());
    final isloading = useState(false);
    final username = useTextEditingController();
    final password = useTextEditingController();
    Future<void> handlePress() async {
      isloading.value = true;

      try {
        VtopClient client = getVtopClient(
          username: username.text,
          password: password.text,
        );
        await vtopClientLogin(client: client);
        _globalUsername = username.text;
        _globalPassword = password.text;
        _globalClient = client;
        two.value = true;
      } catch (e) {
        if (context.mounted) {
          disOnbardingCommonToast(context, e);
        }
      } finally {
        isloading.value = false;
      }
    }

    return Form(
      key: formKey,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          SizedBox(height: 25),
          FTextFormField(
            label: Text("Username"),
            controller: username,
            hint: 'vtop username',
            autovalidateMode: AutovalidateMode.onUserInteraction,
            validator:
                (value) =>
                    1 <= (value?.length ?? 0)
                        ? null
                        : 'Please enter your vtop username.',
          ),
          const SizedBox(height: 10),
          FTextFormField.password(
            controller: password,
            hint: 'vtop password',
            autovalidateMode: AutovalidateMode.onUserInteraction,
            validator:
                (value) =>
                    1 <= (value?.length ?? 0)
                        ? null
                        : 'Please enter your vtop password.',
          ),
          const SizedBox(height: 20),
          !isloading.value
              ? FButton(
                child: const Text('Next'),
                onPress: () {
                  if (formKey.currentState!.validate()) {
                    handlePress();
                    return;
                  }
                },
              )
              : SizedBox(
                height: 20,
                width: 20,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: Colors.black,
                ),
              ),
        ],
      ),
    );
  }
}

class Step2 extends HookConsumerWidget {
  const Step2({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fetching = useState(true);
    final formKey = useMemoized(() => GlobalKey<FormState>());
    final controller = useFRadioSelectMenuTileGroupController<String>();
    final data = useState<SemesterData?>(null);
    Future<void> getSemData() async {
      fetching.value = true;
      try {
        SemesterData semdata = await fetchSemesters(client: _globalClient);
        data.value = semdata;
      } catch (e) {
        if (context.mounted) {
          disOnbardingCommonToast(context, e);
        }
      } finally {
        fetching.value = false;
      }
    }

    useEffect(() {
      Future.microtask(() async {
        getSemData();
      });
      return null;
    }, []);

    if (!fetching.value && data.value != null) {
      return Form(
        key: formKey,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.start,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            FSelectTileGroup(
              selectController: controller,
              label: const Text('Semesters'),
              description: const Text('Select the Semester.'),
              validator:
                  (values) =>
                      values?.isEmpty ?? true ? 'Please select a value.' : null,
              maxHeight: MediaQuery.of(context).size.height * 075,
              children: [
                for (final i in data.value!.semesters)
                  FSelectTile(title: Text(i.name), value: i.id),
              ],
            ),
            const SizedBox(height: 20),
            FButton(
              child: const Text('Save'),
              onPress: () async {
                if (formKey.currentState!.validate()) {
                  try {
                    var user = VtopUserEntity(
                      username: _globalUsername,
                      password: _globalPassword,
                      semid: controller.value.first,
                      isValid: true,
                    );
                    await ref
                        .read(vtopusersutilsProvider.notifier)
                        .vtopUserInitialData(user);
                    ref.invalidate(vtopUserProvider);
                    await ref.read(vClientProvider.future);
                    // ref
                    //     .read(vClientProvider.notifier)
                    //     .replaceVClinet(_globalClient);
                    if (context.mounted) {
                      GoRouter.of(context).goNamed(Paths.timetable);
                    }
                  } catch (e) {
                    log("$e");
                  }
                  return;
                }

                formKey.currentState!.save();
              },
            ),
          ],
        ),
      );
    } else if (fetching.value && data.value == null) {
      return Center(child: CircularProgressIndicator(color: Colors.black));
    } else {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          spacing: 20,
          children: [
            Text("Failed to get semesters data"),
            FButton(
              onPress: () async {
                await getSemData();
              },
              child: const Text('Try again'),
            ),
          ],
        ),
      );
    }
  }
}
