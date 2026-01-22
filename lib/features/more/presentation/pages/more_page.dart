import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class MorePage extends StatelessWidget {
  const MorePage({super.key});

  // Future<void> _launchInBrowser(Uri url) async {
  //   if (!await launchUrl(url, mode: LaunchMode.externalApplication)) {
  //     log('Could not launch $url');
  //   }
  // }

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final Uri url = Uri.parse('https://faculty.synaptic.gg/');
    return Padding(
      padding: const EdgeInsets.all(8.0),
      child: Column(
        spacing: 10,
        children: [
          cardConatiner(
            colors: colors,
            title: 'Marks',
            desc: 'View your Marks',
            child: FButton(
              onPress: () {
                GoRouter.of(context).pushNamed(Paths.marks);
              },
              child: Text("Open"),
            ),
          ),
          cardConatiner(
            colors: colors,
            title: 'Exam Schedule',
            desc: 'View your Exam Schedule',
            child: FButton(
              onPress: () {
                GoRouter.of(context).pushNamed(Paths.examSchedule);
              },
              child: Text("Open"),
            ),
          ),

          VtopCard(),
          // WifiCard(),
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
    final colors = context.theme.colors;
    void handelclick() async {
      isLoading.value = true;
      try {
        await ref.read(vClientProvider.notifier).tryLogin();
        var client = await ref.read(vClientProvider.future);
        if (await fetchIsAuth(client: client)) {
          if (context.mounted) {
            GoRouter.of(context).pushNamed(Paths.vtopweb);
          }
        } else {
          if (context.mounted) {
            disCommonToast(context, Error());
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

    return cardConatiner(
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
      colors: colors,
      title: 'VTOP',
      desc: 'No login requried',
    );
  }
}

Widget cardConatiner({
  Widget? child,
  required FColors colors,
  required String title,
  required String desc,
}) {
  return Container(
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
          title,
          style: TextStyle(color: colors.primary, fontWeight: FontWeight.w600),
        ),
      ),
      subtitle: Center(
        child: Text(desc, style: TextStyle(color: colors.primary)),
      ),
      child: child,
    ),
  );
}

// class WifiCard extends HookConsumerWidget {
//   const WifiCard({super.key});

//   @override
//   Widget build(BuildContext context, WidgetRef ref) {
//     final isLoading = useState(false);
//     final errorMsg = useState<String?>(null);
//     final succesMsg = useState<String?>(null);
//     var wifiDeatils = ref.watch(wifiDetailsProvider);

//     String parseResult(String result) {
//       String output = "..";
//       if (result == "NE") {
//         output = "Looks like you weren't on college Wi-Fi";
//       } else if (result == "NL") {
//         output = "Looks like you were not logged in";
//       } else {
//         output = result;
//       }
//       return output;
//     }

//     void handelLogin(String username, String password) async {
//       var skip = false;
//       try {
//         isLoading.value = true;
//         final isco = await WiFiForIoTPlugin.isConnected();
//         if (isco) {
//           skip = await WiFiForIoTPlugin.forceWifiUsage(true);
//         }
//         if (!skip) {
//           final List<ConnectivityResult> connectivityResult =
//               await (Connectivity().checkConnectivity());
//           if ((connectivityResult.contains(ConnectivityResult.mobile) ||
//               !connectivityResult.contains(ConnectivityResult.wifi))) {
//             if (context.mounted) {
//               showFDialog(
//                 context: context,
//                 builder:
//                     (context, style, animation) => FDialog(
//                       animation: animation,
//                       direction: Axis.horizontal,
//                       title: const Text("Not connected to Wi-Fi."),
//                       body: const Text(
//                         'Please disable mobile data and connect to college Wi-Fi.',
//                       ),
//                       actions: [
//                         FButton(
//                           onPress: () => Navigator.of(context).pop(),
//                           child: const Text('OK'),
//                         ),
//                       ],
//                     ),
//               );
//             }
//             return;
//           }
//         }
//       } catch (e) {
//         ();
//       } finally {
//         isLoading.value = false;
//       }
//       errorMsg.value = null;
//       succesMsg.value = null;
//       isLoading.value = true;
//       username = convertToSuperstring(username);
//       var data = await fetchWifi(username: username, password: password, i: 0);
//       if (!data.$1) {
//         errorMsg.value = parseResult(data.$2);
//       } else {
//         succesMsg.value = data.$2;
//       }
//       isLoading.value = false;
//       if (skip) {
//         await WiFiForIoTPlugin.forceWifiUsage(false);
//       }
//     }

//     void handelLogout() async {
//       errorMsg.value = null;
//       succesMsg.value = null;
//       isLoading.value = true;
//       var skip = false;
//       try {
//         final isco = await WiFiForIoTPlugin.isConnected();
//         if (isco) {
//           skip = await WiFiForIoTPlugin.forceWifiUsage(true);
//         }
//         var data = await fetchWifi(username: "...", password: "...", i: 1);
//         if (!data.$1) {
//           errorMsg.value = parseResult(data.$2);
//         } else {
//           succesMsg.value = data.$2;
//         }
//       } finally {
//         isLoading.value = false;
//         if (skip) {
//           await WiFiForIoTPlugin.forceWifiUsage(false);
//         }
//       }
//     }

//     return Column(
//       crossAxisAlignment: CrossAxisAlignment.stretch,
//       children: [
//         Container(
//           decoration: BoxDecoration(
//             borderRadius: BorderRadius.circular(12),

//             boxShadow: [
//               BoxShadow(
//                 color: MoreColors.cardShadow,
//                 blurRadius: 8,
//                 offset: const Offset(0, 2),
//               ),
//             ],
//           ),
//           child: FCard(
//             title: Stack(
//               children: [
//                 FTappable(
//                   child: Icon(
//                     FIcons.rotateCw,
//                     color: context.theme.colors.primary,
//                   ),
//                   onPress: () {
//                     errorMsg.value = null;
//                     succesMsg.value = null;
//                     isLoading.value = false;
//                   },
//                 ),
//                 Center(
//                   child: Text(
//                     "Wi-Fi",
//                     style: TextStyle(
//                       color: context.theme.colors.primary,
//                       fontWeight: FontWeight.w600,
//                     ),
//                   ),
//                 ),
//                 Positioned(
//                   right: 0,
//                   child: FTappable(
//                     child: Icon(
//                       FIcons.plus,
//                       color: context.theme.colors.primary,
//                     ),
//                     onPress:
//                         () => showAdaptiveDialog(
//                           context: context,
//                           builder: (context) => WifiInput(),
//                         ),
//                   ),
//                 ),
//               ],
//             ),
//             subtitle: Row(
//               mainAxisAlignment: MainAxisAlignment.center,
//               children: [
//                 Text(
//                   "No over-limit on most Wi-Fi networks.",
//                   style: TextStyle(color: context.theme.colors.primary),
//                 ),
//               ],
//             ),
//             child: wifiDeatils.when(
//               data:
//                   (data) => Column(
//                     spacing: 8,
//                     children: [
//                       if (data.$1 == null)
//                         Row(
//                           mainAxisAlignment: MainAxisAlignment.center,
//                           children: [
//                             Text(
//                               "Add your wifi details by clicking  ",
//                               style: TextStyle(
//                                 color: context.theme.colors.primary,
//                               ),
//                             ),
//                             FTappable(
//                               child: Icon(
//                                 FIcons.plus,
//                                 color: context.theme.colors.primary,
//                               ),
//                               onPress:
//                                   () => showAdaptiveDialog(
//                                     context: context,
//                                     builder: (context) => WifiInput(),
//                                   ),
//                             ),
//                           ],
//                         ),
//                       if ((data.$1 != null))
//                         Center(
//                           child: Text(
//                             data.$1 ?? "",
//                             style: TextStyle(
//                               color: context.theme.colors.primary,
//                             ),
//                           ),
//                         ),
//                       if (succesMsg.value != null)
//                         Row(
//                           mainAxisAlignment: MainAxisAlignment.center,
//                           spacing: 8,
//                           children: [
//                             Icon(
//                               FIcons.circleCheckBig,
//                               color: MoreColors.successText,
//                             ),
//                             Expanded(
//                               child: Text(
//                                 succesMsg.value!,
//                                 maxLines: 2,
//                                 style: TextStyle(color: MoreColors.successText),
//                               ),
//                             ),
//                           ],
//                         ),
//                       if (errorMsg.value != null)
//                         Row(
//                           mainAxisAlignment: MainAxisAlignment.center,
//                           spacing: 8,
//                           children: [
//                             Icon(FIcons.circleX, color: MoreColors.errorText),
//                             Expanded(
//                               child: Text(
//                                 errorMsg.value!,
//                                 maxLines: 2,
//                                 style: TextStyle(color: MoreColors.errorText),
//                               ),
//                             ),
//                           ],
//                         ),
//                       !isLoading.value
//                           ? Row(
//                             spacing: 8,
//                             children: [
//                               Expanded(
//                                 child: FButton(
//                                   onPress: () {
//                                     if (data.$1 != null && data.$2 != null) {
//                                       handelLogin(data.$1!, data.$2!);
//                                     }
//                                   },
//                                   child: Text("Login"),
//                                 ),
//                               ),
//                               Expanded(
//                                 child: FButton(
//                                   onPress: () {
//                                     handelLogout();
//                                   },
//                                   child: Text("Logout"),
//                                 ),
//                               ),
//                             ],
//                           )
//                           : const FCircularProgress.pinwheel(),
//                     ],
//                   ),
//               error:
//                   (e, stackTrace) =>
//                       Text("$e", style: TextStyle(color: MoreColors.errorText)),
//               loading:
//                   () => const SizedBox(
//                     height: 20,
//                     width: 20,
//                     child: CircularProgressIndicator(
//                       strokeWidth: 2,
//                       color: MoreColors.infoBorder,
//                     ),
//                   ),
//             ),
//           ),
//         ),
//       ],
//     );
//   }
// }

// String convertToSuperstring(String k) {
//   Random random = Random();
//   final Map<String, List<String>> map = {
//     "1": ["1️⃣", "¹"],
//     "2": ["2️⃣", "²"],
//     "3": ["3️⃣", "³"],
//     "4": ["4️⃣", "⁴"],
//     "5": ["5️⃣", "⁵"],
//     "6": ["6️⃣", "⁶"],
//     "7": ["7️⃣", "⁷"],
//     "8": ["8️⃣", "⁸"],
//     "9": ["9️⃣", "⁹"],
//     "0": ["0️⃣", "⁰"],
//   };

//   List<String> list = k.split('');
//   var out = k;
//   while (out == k) {
//     out = list
//         .map((val) {
//           if (map.containsKey(val)) {
//             final options = [0, 2, 2, 2, 1, 1, 1, 1];
//             int choice = options[random.nextInt(options.length)];
//             if (choice == 0 || choice == 1) {
//               return map[val]![choice];
//             } else {
//               return val;
//             }
//           }
//           return val;
//         })
//         .join("");
//   }

//   return out;
// }

// class WifiInput extends HookConsumerWidget {
//   const WifiInput({super.key});

//   @override
//   Widget build(BuildContext context, WidgetRef ref) {
//     final formKey = useMemoized(() => GlobalKey<FormState>());
//     final isloading = useState(false);
//     final username = useTextEditingController();
//     final password = useTextEditingController();

//     Future<void> handelClick() async {
//       isloading.value = true;
//       await ref.read(vtopusersutilsProvider.notifier).saveWifiDetails((
//         username.text,
//         password.text,
//       ));
//       var _ = ref.refresh(wifiDetailsProvider);
//       isloading.value = false;
//     }

//     return FDialog(
//       title: Text(
//         "Wi-Fi credentials",
//         style: TextStyle(color: context.theme.colors.primary),
//       ),
//       body: Form(
//         key: formKey,
//         child: Column(
//           mainAxisSize: MainAxisSize.min,
//           mainAxisAlignment: MainAxisAlignment.start,
//           children: [
//             SizedBox(height: 25),
//             FTextFormField(
//               label: Text(
//                 "Username",
//                 style: TextStyle(color: context.theme.colors.primary),
//               ),
//               controller: username,
//               hint: 'wi-fi username',
//               autovalidateMode: AutovalidateMode.onUserInteraction,
//               validator:
//                   (value) =>
//                       1 <= (value?.length ?? 0)
//                           ? null
//                           : 'Please enter your wi-fi username.',
//             ),
//             const SizedBox(height: 10),
//             FTextFormField.password(
//               controller: password,
//               hint: 'wi-fi password',
//               autovalidateMode: AutovalidateMode.onUserInteraction,
//               validator:
//                   (value) =>
//                       1 <= (value?.length ?? 0)
//                           ? null
//                           : 'Please enter your wi-fi password.',
//             ),
//           ],
//         ),
//       ),
//       actions: [
//         if (!isloading.value)
//           FButton(
//             style: FButtonStyle.primary(),
//             onPress: () async {
//               if (formKey.currentState!.validate()) {
//                 await handelClick();
//                 if (context.mounted) {
//                   Navigator.of(context).pop();
//                 }
//                 return;
//               }
//             },
//             child: const Text('Save'),
//           ),
//         if (isloading.value)
//           const SizedBox(
//             height: 20,
//             width: 20,
//             child: CircularProgressIndicator(
//               strokeWidth: 2,
//               color: MoreColors.infoBorder,
//             ),
//           ),
//         FButton(
//           style: FButtonStyle.outline(),
//           onPress: () => Navigator.of(context).pop(),
//           child: const Text('Cancel'),
//         ),
//       ],
//     );
//   }
// }
