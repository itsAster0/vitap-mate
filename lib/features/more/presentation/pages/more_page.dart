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

    final colors = context.theme.colors;
    // Each group gets its own icon tone so a tile can be found by colour and
    // position without reading every label.
    final sections = [
      (
        'Results',
        colors.app.accentTone,
        [
          _Tool(
            FLucideIcons.clipboardList,
            'Marks',
            'Scores so far',
            () => push(Paths.marks),
          ),
          _Tool(
            FLucideIcons.graduationCap,
            'Grades',
            'This semester',
            () => push(Paths.grades),
          ),
          _Tool(
            FLucideIcons.history,
            'Grade History',
            'All semesters',
            () => push(Paths.gradeHistory),
          ),
          _Tool(
            FLucideIcons.calculator,
            'GPA Planner',
            'Plan your CGPA',
            () => push(Paths.gpaCalculator),
          ),
        ],
      ),
      (
        'Schedule',
        colors.app.warning,
        [
          _Tool(
            FLucideIcons.calendarDays,
            'Exams',
            'Schedule & seats',
            () => push(Paths.examSchedule),
          ),
          _Tool(
            FLucideIcons.partyPopper,
            'Calendar',
            'Holidays & exams',
            () => push(Paths.academicCalendar),
          ),
          _Tool(
            FLucideIcons.bookOpen,
            'Course Page',
            'Lecture notes',
            () => push(Paths.coursePage),
          ),
        ],
      ),
      (
        'Campus',
        colors.app.lab,
        [
          _Tool(
            FLucideIcons.scanFace,
            'Biometric',
            'Entry logs',
            () => push(Paths.biometricHistory),
          ),
          _Tool(
            FLucideIcons.doorOpen,
            'Outing',
            'Leave & passes',
            () => push(Paths.outing),
          ),
        ],
      ),
    ];

    var index = 0;
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
          for (final (title, tone, tools) in sections) ...[
            SectionHeader(title: title),
            GridView.count(
              crossAxisCount: 2,
              shrinkWrap: true,
              padding: EdgeInsets.zero,
              physics: const NeverScrollableScrollPhysics(),
              mainAxisSpacing: Space.sm + 2,
              crossAxisSpacing: Space.sm + 2,
              // Icon beside the text keeps each tile to about two lines tall.
              mainAxisExtent: 66,
              children: [
                for (final tool in tools)
                  EnterFade(
                    index: index++,
                    child: _ToolTile(tool: tool, tone: tone),
                  ),
              ],
            ),
          ],
          const SectionHeader(title: 'VTOP'),
          FTileGroup(
            children: [
              FTile(
                prefix: const Icon(FLucideIcons.externalLink),
                title: const Text("Open VTOP"),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: openVtop,
              ),
            ],
          ),
          if (recentPages.isNotEmpty)
            _RecentVtopPages(
              pages: recentPages,
              onOpen: (page) => openVtop(page.url),
            ),
          if (cookieBridgeAvailable == true) ...[
            const SizedBox(height: Space.lg),
            FTileGroup(
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.puzzle),
                  title: const Text("Chrome Extension"),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => push(Paths.chromeExtension),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

class _Tool {
  const _Tool(this.icon, this.title, this.subtitle, this.onPress);

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onPress;
}

class _ToolTile extends StatelessWidget {
  const _ToolTile({required this.tool, required this.tone});

  final _Tool tool;
  final Tone tone;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Surface(
      onPress: tool.onPress,
      semanticsLabel: tool.title,
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
              color: tone.subtle,
              borderRadius: BorderRadius.circular(Radii.sm + 2),
            ),
            child: Icon(tool.icon, size: 17, color: tone.onSubtle),
          ),
          const SizedBox(width: Space.sm + 2),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  tool.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: colors.foreground,
                  ),
                ),
                Text(
                  tool.subtitle,
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
      padding: const EdgeInsets.only(top: Space.sm + 2),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        clipBehavior: Clip.none,
        child: Row(
          spacing: Space.sm,
          children: [
            Padding(
              padding: const EdgeInsets.only(left: 2, right: Space.xs),
              child: Text(
                'RECENT',
                style: typography.body.xs.copyWith(
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.6,
                  color: colors.mutedForeground,
                ),
              ),
            ),
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
