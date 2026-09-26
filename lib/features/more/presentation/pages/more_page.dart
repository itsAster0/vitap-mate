import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/vtop_webview_pages.dart';
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
    final recentPages = ref.watch(vtopRecentPagesProvider);

    final tools = [
      (FLucideIcons.clipboardList, 'Marks', 'Scores so far', Paths.marks),
      (FLucideIcons.graduationCap, 'Grades', 'This semester', Paths.grades),
      (
        FLucideIcons.history,
        'Grade History',
        'All semesters',
        Paths.gradeHistory,
      ),
      (
        FLucideIcons.calculator,
        'GPA Planner',
        'Plan your CGPA',
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
            // Icon beside the text keeps each tile to about two lines tall.
            mainAxisExtent: 66,
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
          if (recentPages.isNotEmpty)
            _RecentVtopPages(
              pages: recentPages,
              onOpen: (page) => openVtop(page.url),
            ),
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
      padding: const EdgeInsets.symmetric(
        horizontal: Space.md,
        vertical: Space.sm + 2,
      ),
      child: Row(
        children: [
          Container(
            width: 34,
            height: 34,
            decoration: BoxDecoration(
              color: colors.app.accentTone.subtle,
              borderRadius: BorderRadius.circular(Radii.sm + 2),
            ),
            child: Icon(icon, size: 17, color: colors.app.accentTone.onSubtle),
          ),
          const SizedBox(width: Space.sm + 2),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
                Text(
                  subtitle,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Pages last opened inside VTOP, one tap away from More.
class _RecentVtopPages extends StatelessWidget {
  const _RecentVtopPages({required this.pages, required this.onOpen});

  final List<VtopPage> pages;
  final ValueChanged<VtopPage> onOpen;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.md),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        clipBehavior: Clip.none,
        child: Row(
          spacing: Space.sm,
          children: [
            for (final page in pages)
              PressScale(
                onPress: () => onOpen(page),
                semanticsLabel: 'Open ${page.title} in VTOP',
                scale: 0.96,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: colors.card,
                    borderRadius: BorderRadius.circular(Radii.pill),
                    border: Border.all(color: colors.border),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: Space.md,
                      vertical: Space.sm,
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      spacing: 6,
                      children: [
                        Icon(
                          FLucideIcons.history,
                          size: 14,
                          color: colors.mutedForeground,
                        ),
                        Text(
                          page.title,
                          style: typography.body.sm.copyWith(
                            color: colors.foreground,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
