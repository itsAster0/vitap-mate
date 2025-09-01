import 'dart:developer';
import 'dart:math' hide log;
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/core/utils/users/wifi_details.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:url_launcher/url_launcher.dart';

class MorePage extends StatelessWidget {
  const MorePage({super.key});

  Future<void> _launchInBrowser(Uri url) async {
    if (!await launchUrl(url, mode: LaunchMode.externalApplication)) {
      log('Could not launch $url');
    }
  }

  @override
  Widget build(BuildContext context) {
    final Uri url = Uri.parse('https://faculty.synaptic.gg/');
    return Padding(
      padding: const EdgeInsets.all(8.0),
      child: Column(
        spacing: 10,
        children: [
          FTile(
            prefixIcon: Icon(FIcons.bookCheck),
            title: Text(
              'Faculty Directory',
              style: TextStyle(color: context.theme.colors.primary),
            ),
            suffixIcon: Icon(
              FIcons.chevronRight,
              color: context.theme.colors.primary,
            ),
            onPress: () {
              _launchInBrowser(url);
            },
          ),
          FTile(
            prefixIcon: Icon(FIcons.bookCheck),
            title: Text(
              'Marks',
              style: TextStyle(color: context.theme.colors.primary),
            ),
            suffixIcon: Icon(
              FIcons.chevronRight,
              color: context.theme.colors.primary,
            ),
            onPress: () {
              GoRouter.of(context).pushNamed(Paths.marks);
            },
          ),
          FTile(
            prefixIcon: Icon(FIcons.school),
            title: Text(
              'Exam Schedule',
              style: TextStyle(color: context.theme.colors.primary),
            ),
            suffixIcon: Icon(
              FIcons.chevronRight,
              color: context.theme.colors.primary,
            ),
            onPress: () {
              GoRouter.of(context).pushNamed(Paths.examSchedule);
            },
          ),
          VtopCard(),
          WifiCard(),
        ],
      ),
    );
  }
}

class VtopCard extends HookConsumerWidget {
  const VtopCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isLoading = useState(false);

    void handelclick() async {
      isLoading.value = true;
      try {
        await ref.read(vClientProvider.notifier).tryLogin();
        var client = await ref.read(vClientProvider.future);
        if (await fetchIsAuth(client: client)) {
          if (context.mounted) {
            GoRouter.of(context).pushNamed(Paths.vtopweb);
          }
        }
      } catch (e) {
        if (context.mounted) {
          disCommonToast(context, e);
        }
      } finally {
        isLoading.value = false;
      }
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),

            boxShadow: [
              BoxShadow(
                color: MoreColors.cardShadow,
                blurRadius: 8,
                offset: const Offset(0, 2),
              ),
            ],
          ),
          child: FCard(
            title: Center(
              child: Text(
                "Vtop",
                style: TextStyle(
                  color: context.theme.colors.primary,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            subtitle: Center(
              child: Text(
                "No login required",
                style: TextStyle(color: context.theme.colors.primary),
              ),
            ),
            child:
                !isLoading.value
                    ? FButton(
                      onPress: () async {
                        handelclick();
                      },
                      child: Text("Open"),
                    )
                    : Center(
                      child: const SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: MoreColors.infoBorder,
                        ),
                      ),
                    ),
          ),
        ),
      ],
    );
  }
}

class WifiCard extends HookConsumerWidget {
  const WifiCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isLoading = useState(false);
    final errorMsg = useState<String?>(null);
    final succesMsg = useState<String?>(null);
    var wifiDeatils = ref.watch(wifiDetailsProvider);

    String parseResult(String result) {
      String output = "..";
      if (result == "NE") {
        output = "Looks like you weren't on college Wi-Fi";
      } else if (result == "NL") {
        output = "Looks like you were not logged in";
      } else {
        output = result;
      }
      return output;
    }

    void handelLogin(String username, String password) async {
      errorMsg.value = null;
      succesMsg.value = null;
      isLoading.value = true;
      username = convertToSuperstring(username);
      var data = await fetchWifi(username: username, password: password, i: 0);
      if (!data.$1) {
        errorMsg.value = parseResult(data.$2);
      } else {
        succesMsg.value = data.$2;
      }
      isLoading.value = false;
    }

    void handelLogout() async {
      errorMsg.value = null;
      succesMsg.value = null;
      isLoading.value = true;
      var data = await fetchWifi(username: "...", password: "...", i: 1);
      if (!data.$1) {
        errorMsg.value = parseResult(data.$2);
      } else {
        succesMsg.value = data.$2;
      }
      isLoading.value = false;
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),

            boxShadow: [
              BoxShadow(
                color: MoreColors.cardShadow,
                blurRadius: 8,
                offset: const Offset(0, 2),
              ),
            ],
          ),
          child: FCard(
            title: Stack(
              children: [
                FTappable(
                  child: Icon(FIcons.rotateCw),
                  onPress: () {
                    errorMsg.value = null;
                    succesMsg.value = null;
                    isLoading.value = false;
                  },
                ),
                Center(
                  child: Text(
                    "Wi-fi",
                    style: TextStyle(
                      color: context.theme.colors.primary,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                Positioned(
                  right: 0,
                  child: FTappable(
                    child: Icon(FIcons.plus),
                    onPress:
                        () => showAdaptiveDialog(
                          context: context,
                          builder: (context) => WifiInput(),
                        ),
                  ),
                ),
              ],
            ),
            subtitle: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Text(
                  "No over-limit on most Wi-Fi networks.",
                  style: TextStyle(color: context.theme.colors.primary),
                ),
              ],
            ),
            child: wifiDeatils.when(
              data:
                  (data) => Column(
                    spacing: 8,
                    children: [
                      if (data.$1 == null)
                        Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Text(
                              "Add your wifi details by clicking  ",
                              style: TextStyle(
                                color: context.theme.colors.primary,
                              ),
                            ),
                            FTappable(
                              child: Icon(FIcons.plus),
                              onPress:
                                  () => showAdaptiveDialog(
                                    context: context,
                                    builder: (context) => WifiInput(),
                                  ),
                            ),
                          ],
                        ),
                      if ((data.$1 != null))
                        Center(
                          child: Text(
                            data.$1 ?? "",
                            style: TextStyle(
                              color: context.theme.colors.primary,
                            ),
                          ),
                        ),
                      if (succesMsg.value != null)
                        Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          spacing: 8,
                          children: [
                            Icon(
                              FIcons.circleCheckBig,
                              color: MoreColors.successText,
                            ),
                            Expanded(
                              child: Text(
                                succesMsg.value!,
                                maxLines: 2,
                                style: TextStyle(color: MoreColors.successText),
                              ),
                            ),
                          ],
                        ),
                      if (errorMsg.value != null)
                        Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          spacing: 8,
                          children: [
                            Icon(FIcons.circleX, color: MoreColors.errorText),
                            Expanded(
                              child: Text(
                                errorMsg.value!,
                                maxLines: 2,
                                style: TextStyle(color: MoreColors.errorText),
                              ),
                            ),
                          ],
                        ),
                      !isLoading.value
                          ? Row(
                            spacing: 8,
                            children: [
                              Expanded(
                                child: FButton(
                                  onPress: () {
                                    if (data.$1 != null && data.$2 != null) {
                                      handelLogin(data.$1!, data.$2!);
                                    }
                                  },
                                  child: Text("Login"),
                                ),
                              ),
                              Expanded(
                                child: FButton(
                                  onPress: () {
                                    handelLogout();
                                  },
                                  child: Text("Logout"),
                                ),
                              ),
                            ],
                          )
                          : const FProgress(),
                    ],
                  ),
              error:
                  (e, stackTrace) =>
                      Text("$e", style: TextStyle(color: MoreColors.errorText)),
              loading:
                  () => const SizedBox(
                    height: 20,
                    width: 20,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: MoreColors.infoBorder,
                    ),
                  ),
            ),
          ),
        ),
      ],
    );
  }
}

String convertToSuperstring(String k) {
  final Map<String, String> map = {
    "1": "¹",
    "2": "²",
    "3": "³",
    "4": "⁴",
    "5": "⁵",
    "6": "⁶",
    "7": "⁷",
    "8": "⁸",
    "9": "⁹",
    "0": "⁰",
  };

  List<String> n = k.split('');
  Random random = Random();
  int runs = 0;

  do {
    runs = random.nextInt(n.length + 1);
  } while (runs == 0 || runs == 1);

  for (int i = 0; i <= runs; i++) {
    int c = random.nextInt(n.length);
    String charAtIndex = n[c];
    int? parsedChar = int.tryParse(charAtIndex);

    if (parsedChar != null) {
      if (map.containsKey(charAtIndex)) {
        n[c] = map[charAtIndex]!;
      }
    }
  }
  return n.join('');
}

class WifiInput extends HookConsumerWidget {
  const WifiInput({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final formKey = useMemoized(() => GlobalKey<FormState>());
    final isloading = useState(false);
    final username = useTextEditingController();
    final password = useTextEditingController();

    Future<void> handelClick() async {
      isloading.value = true;
      await ref.read(vtopusersutilsProvider.notifier).saveWifiDetails((
        username.text,
        password.text,
      ));
      var _ = ref.refresh(wifiDetailsProvider);
      isloading.value = false;
    }

    return FDialog(
      title: Text(
        "Wi-fi credentials",
        style: TextStyle(color: context.theme.colors.primary),
      ),
      body: Form(
        key: formKey,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.start,
          children: [
            SizedBox(height: 25),
            FTextFormField(
              label: Text(
                "Username",
                style: TextStyle(color: context.theme.colors.primary),
              ),
              controller: username,
              hint: 'wi-fi username',
              autovalidateMode: AutovalidateMode.onUserInteraction,
              validator:
                  (value) =>
                      1 <= (value?.length ?? 0)
                          ? null
                          : 'Please enter your wi-fi username.',
            ),
            const SizedBox(height: 10),
            FTextFormField.password(
              controller: password,
              hint: 'wi-fi password',
              autovalidateMode: AutovalidateMode.onUserInteraction,
              validator:
                  (value) =>
                      1 <= (value?.length ?? 0)
                          ? null
                          : 'Please enter your wi-fi password.',
            ),
          ],
        ),
      ),
      actions: [
        if (!isloading.value)
          FButton(
            style: FButtonStyle.primary,
            onPress: () async {
              if (formKey.currentState!.validate()) {
                await handelClick();
                if (context.mounted) {
                  Navigator.of(context).pop();
                }
                return;
              }
            },
            child: const Text('Save'),
          ),
        if (isloading.value)
          const SizedBox(
            height: 20,
            width: 20,
            child: CircularProgressIndicator(
              strokeWidth: 2,
              color: MoreColors.infoBorder,
            ),
          ),
        FButton(
          style: FButtonStyle.outline,
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
