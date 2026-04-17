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

class MorePage extends HookConsumerWidget {
  const MorePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 8, 8, 0),
      child: SingleChildScrollView(
        child: Column(
          spacing: 10,
          children: [
            FTileGroup(
              label: const Text("Academic"),
              divider: FItemDivider.indented,
              children: [
                FTile(
                  prefix: const Icon(Icons.grading_outlined),
                  title: const Text('Marks'),
                  subtitle: const Text('View your Marks'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.marks);
                  },
                ),
                FTile(
                  prefix: const Icon(Icons.school_outlined),
                  title: const Text('Grades'),
                  subtitle: const Text('View grades with detailed marks'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.grades);
                  },
                ),
                FTile(
                  prefix: const Icon(Icons.history_outlined),
                  title: const Text('Grade History'),
                  subtitle: const Text('View complete grade history'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.gradeHistory);
                  },
                ),
                FTile(
                  prefix: const Icon(Icons.event_note_outlined),
                  title: const Text('Exam Schedule'),
                  subtitle: const Text('View your Exam Schedule'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.examSchedule);
                  },
                ),
              ],
            ),
            const VtopCard(),
          ],
        ),
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

    Future<void> handleClick() async {
      isLoading.value = true;
      try {
        await ref.read(vClientProvider.notifier).tryLogin();
        final client = await ref.read(vClientProvider.future);
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
      child: !isLoading.value
          ? FButton(
              onPress: () async {
                await handleClick();
              },
              child: const Text("Open"),
            )
          : const Center(
              child: SizedBox(
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
