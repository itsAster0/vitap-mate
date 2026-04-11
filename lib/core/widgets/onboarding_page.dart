import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:forui_hooks/forui_hooks.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/registration_number.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';

import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

late VtopClient _globalClient;
late String _globalUsername;
late String _globalPassword;
String? _globalEmail;
PendingSignInResult? _globalPending;

class OnboardingPage extends HookConsumerWidget {
  const OnboardingPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final stepTwo = useState(false);
    return FScaffold(
      resizeToAvoidBottomInset: true,
      header: FHeader.nested(title: Text("Vitap Mate"), suffixes: []),
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
    final email = useTextEditingController();
    Future<void> handleGooglePress() async {
      isloading.value = true;
      try {
        final collegeEmail =
            await (await ref.read(
              appServicesProvider.future,
            )).authRepository.signInWithGoogleEmail();
        if (username.text.trim().isEmpty) {
          username.text = deriveRegistrationNumber(collegeEmail);
        }
        email.text = collegeEmail;
        _globalEmail = collegeEmail;
      } catch (e) {
        if (context.mounted) {
          disOnbardingCommonToast(context, e);
        }
      } finally {
        isloading.value = false;
      }
    }

    Future<void> handlePress() async {
      isloading.value = true;

      try {
        if (_globalEmail == null || username.text.trim().isEmpty) {
          throw const FormatException('Continue with your VITAP email first.');
        }
        final pending = await (await ref.read(
          appServicesProvider.future,
        )).authRepository.verifyUsernameAndPassword(
          email: _globalEmail,
          username: username.text.trim(),
          password: password.text,
        );
        VtopClient client = getVtopClient(
          username: pending.registrationNumber,
          password: pending.password,
          cookie: pending.cookie,
        );
        _globalUsername = pending.registrationNumber;
        _globalPassword = pending.password;
        _globalClient = client;
        _globalPending = pending;
        two.value = true;
      } catch (e) {
        if (context.mounted) {
          disOnbardingCommonToast(context, e);
        }
      } finally {
        isloading.value = false;
      }
    }

    return SingleChildScrollView(
      child: Form(
        key: formKey,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.start,
          children: [
            SizedBox(height: 25),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: FAlert(
                icon: const Icon(FIcons.mail),
                title: const Text('Use your college email'),
                subtitle: const Text(
                  'Continue with your VITAP email and allow Gmail access for OTP.',
                ),
              ),
            ),
            const SizedBox(height: 10),
            if (!isloading.value)
              FButton(
                variant: FButtonVariant.outline,
                onPress: handleGooglePress,
                child: const Text('Continue with VITAP Email'),
              ),
            const SizedBox(height: 10),
            FTextFormField(
              label: const Text("College Email"),
              control: FTextFieldControl.managed(controller: email),
              hint: 'vitapstudent.ac.in email',
              enabled: false,
              autovalidateMode: AutovalidateMode.onUserInteraction,
              validator:
                  (value) =>
                      1 <= (value?.trim().length ?? 0)
                          ? null
                          : 'Continue with your VITAP email first.',
            ),
            const SizedBox(height: 10),
            FTextFormField(
              label: const Text("VTOP Username"),
              control: FTextFieldControl.managed(controller: username),
              hint: 'registration number',
              autovalidateMode: AutovalidateMode.onUserInteraction,
              validator:
                  (value) =>
                      1 <= (value?.length ?? 0)
                          ? null
                          : 'Enter your VTOP username.',
            ),
            const SizedBox(height: 10),
            FTextFormField.password(
              control: FTextFieldControl.managed(controller: password),
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
                    color: context.theme.colors.primary,
                  ),
                ),
            const SizedBox(height: 16),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: FAlert(
                variant: FAlertVariant.destructive,
                icon: const Icon(FIcons.shieldCheck),
                title: const Text('Use the Play Store app'),
                subtitle: const Text(
                  'Make sure you installed Vitap Mate from the Play Store before signing in.',
                ),
              ),
            ),
          ],
        ),
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
    final controller = useFRadioMultiValueNotifier<String>();
    final data = useState<SemesterData?>(null);
    Future<void> getSemData() async {
      fetching.value = true;
      try {
        data.value =
            _globalPending?.semesters ??
            await fetchSemesters(client: _globalClient);
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
              control: FMultiValueControl.managedRadio(controller: controller),
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
                    final pending = _globalPending;
                    if (pending != null) {
                      await (await ref.read(
                        appServicesProvider.future,
                      )).authRepository.saveVerifiedAccount(
                        pending: pending,
                        selectedSemesterId: controller.value.first,
                      );
                    }
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
                    if (context.mounted) {
                      disOnbardingCommonToast(context, e);
                    }
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
      return Center(
        child: CircularProgressIndicator(color: context.theme.colors.primary),
      );
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
