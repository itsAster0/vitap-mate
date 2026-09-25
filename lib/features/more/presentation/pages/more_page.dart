import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

class MorePage extends HookConsumerWidget {
  const MorePage({super.key});

  static const _coursePageUrl = 'academics/common/StudentCoursePage';
  static const _generalOutingUrl = 'hostel/StudentGeneralOuting';
  static const _weekendOutingUrl = 'hostel/StudentWeekendOuting';

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cookieBridgeAvailable = ref
        .watch(fcmCookieBridgeAvailableProvider)
        .when(
          data: (available) => available,
          loading: () => false,
          error: (_, _) => false,
        );

    void openVtop([String? menuUrl]) {
      GoRouter.of(context).pushNamed(Paths.vtopweb, extra: menuUrl);
    }

    void push(String name) => GoRouter.of(context).pushNamed(name);

    final tools = [
      (FLucideIcons.clipboardList, 'Marks', 'Assessment scores', Paths.marks),
      (FLucideIcons.graduationCap, 'Grades', 'This semester', Paths.grades),
      (
        FLucideIcons.history,
        'Grade History',
        'CGPA & all courses',
        Paths.gradeHistory,
      ),
      (
        FLucideIcons.calculator,
        'GPA Planner',
        'Project your CGPA',
        Paths.gpaCalculator,
      ),
      (
        FLucideIcons.calendarDays,
        'Exams',
        'Schedule & seats',
        Paths.examSchedule,
      ),
      (
        FLucideIcons.scanFace,
        'Biometric',
        'Entry logs',
        Paths.biometricHistory,
      ),
    ];

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        Space.sm,
        Space.xs,
        Space.sm,
        Space.xl,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SectionHeader(title: 'Academics'),
          GridView.count(
            crossAxisCount: 2,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            mainAxisSpacing: Space.sm + 2,
            crossAxisSpacing: Space.sm + 2,
            childAspectRatio: 1.45,
            children: [
              for (final (i, (icon, title, subtitle, route)) in tools.indexed)
                EnterFade(
                  index: i,
                  child: _ToolTile(
                    icon: icon,
                    title: title,
                    subtitle: subtitle,
                    onPress: () => push(route),
                  ),
                ),
            ],
          ),
          const SectionHeader(title: 'VTOP'),
          FTileGroup(
            children: [
              FTile(
                prefix: const Icon(FLucideIcons.externalLink),
                title: const Text("Open VTOP"),
                subtitle: const Text("Open the VTOP portal without auto login"),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: openVtop,
              ),
              FTile(
                prefix: const Icon(FLucideIcons.book),
                title: const Text("Course Page"),
                subtitle: const Text("Open the VTOP course page"),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => openVtop(_coursePageUrl),
              ),
              FTile(
                prefix: const Icon(FLucideIcons.doorOpen),
                title: const Text("General Outing"),
                subtitle: const Text("Open general outing in VTOP"),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => openVtop(_generalOutingUrl),
              ),
              FTile(
                prefix: const Icon(FLucideIcons.luggage),
                title: const Text("Weekend Outing"),
                subtitle: const Text("Open weekend outing in VTOP"),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => openVtop(_weekendOutingUrl),
              ),
            ],
          ),
          if (cookieBridgeAvailable == true)
            const SectionHeader(title: 'Browser extension'),
          if (cookieBridgeAvailable == true)
            FTileGroup(
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.puzzle),
                  title: const Text("Chrome Extension"),
                  subtitle: const Text(
                    "Install VITAP Mate auto login on your computer",
                  ),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => push(Paths.chromeExtension),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

class _ToolTile extends StatelessWidget {
  const _ToolTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onPress,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Surface(
      onPress: onPress,
      semanticsLabel: title,
      padding: const EdgeInsets.all(Space.md + 2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: colors.app.accentTone.subtle,
              borderRadius: BorderRadius.circular(Radii.sm + 2),
            ),
            child: Icon(icon, size: 18, color: colors.app.accentTone.onSubtle),
          ),
          const Spacer(),
          Text(
            title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: typography.body.md.copyWith(
              fontWeight: FontWeight.w600,
              color: colors.foreground,
            ),
          ),
          Text(
            subtitle,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: typography.body.xs.copyWith(color: colors.mutedForeground),
          ),
        ],
      ),
    );
  }
}
