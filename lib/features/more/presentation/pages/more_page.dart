import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';

class MorePage extends HookConsumerWidget {
  const MorePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 8, 8, 0),
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 12,
          children: [
            FTileGroup(
              label: const Text("Academic Tools"),
              divider: FItemDivider.indented,
              children: [
                FTile(
                  prefix: const Icon(Icons.grading_outlined),
                  title: const Text('Marks'),
                  subtitle: const Text('View your marks'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () => GoRouter.of(context).pushNamed(Paths.marks),
                ),
                FTile(
                  prefix: const Icon(Icons.school_outlined),
                  title: const Text('Grades'),
                  subtitle: const Text('View grades with detailed marks'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () => GoRouter.of(context).pushNamed(Paths.grades),
                ),
                FTile(
                  prefix: const Icon(Icons.history_outlined),
                  title: const Text('Grade History'),
                  subtitle: const Text('View complete grade history'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress:
                      () => GoRouter.of(context).pushNamed(Paths.gradeHistory),
                ),
                FTile(
                  prefix: const Icon(Icons.event_note_outlined),
                  title: const Text('Exam Schedule'),
                  subtitle: const Text('View your exam schedule'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress:
                      () => GoRouter.of(context).pushNamed(Paths.examSchedule),
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
    final colors = context.theme.colors;

    return cardConatiner(
      child: FButton(
        onPress: () => GoRouter.of(context).pushNamed(Paths.vtopweb),
        child: const Text("Open"),
      ),
      colors: colors,
      title: 'VTOP',
      desc: 'Open the VTOP',
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
