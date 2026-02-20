import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/features/more/presentation/widgets/wifi_card.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class MorePage extends HookConsumerWidget {
  const MorePage({super.key});

  // Future<void> _launchInBrowser(Uri url) async {
  //   if (!await launchUrl(url, mode: LaunchMode.externalApplication)) {
  //     log('Could not launch $url');
  //   }
  // }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final Uri url = Uri.parse('https://faculty.kryxen.dev/');
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
          if (ref.watch(wificardSettingProvider)) WifiCard(),
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
