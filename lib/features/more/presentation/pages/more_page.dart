import 'dart:io';

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/paths.dart';

class MorePage extends HookConsumerWidget {
  const MorePage({super.key});

  static const _coursePageUrl = 'academics/common/StudentCoursePage';
  static const _generalOutingUrl = 'hostel/StudentGeneralOuting';
  static const _weekendOutingUrl = 'hostel/StudentWeekendOuting';

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    void openVtop([String? menuUrl]) {
      GoRouter.of(context).pushNamed(Paths.vtopweb, extra: menuUrl);
    }

    return SingleChildScrollView(
      padding: EdgeInsets.all(4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 12,
        children: [
          FTileGroup(
            label: const Text("Academic Tools"),
            divider: FItemDivider.indented,
            children: [
              FTile(
                prefix: const Icon(FLucideIcons.clipboardList),
                title: const Text('Marks'),
                subtitle: const Text('View your marks'),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => GoRouter.of(context).pushNamed(Paths.marks),
              ),
              FTile(
                prefix: const Icon(FLucideIcons.graduationCap),
                title: const Text('Grades'),
                subtitle: const Text('View grades with detailed marks'),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => GoRouter.of(context).pushNamed(Paths.grades),
              ),
              FTile(
                prefix: const Icon(FLucideIcons.history),
                title: const Text('Grade History'),
                subtitle: const Text('View complete grade history'),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () =>
                    GoRouter.of(context).pushNamed(Paths.gradeHistory),
              ),
              // FTile(
              //   prefix: const Icon(FLucideIcons.calculator),
              //   title: const Text('GPA / CGPA Calculator'),
              //   subtitle: const Text('Plan your grades and project CGPA'),
              //   suffix: const Icon(FLucideIcons.chevronRight),
              //   onPress: () =>
              //       GoRouter.of(context).pushNamed(Paths.gpaCalculator),
              // ),
              FTile(
                prefix: const Icon(FLucideIcons.calendarDays),
                title: const Text('Exam Schedule'),
                subtitle: const Text('View your exam schedule'),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () =>
                    GoRouter.of(context).pushNamed(Paths.examSchedule),
              ),
              FTile(
                prefix: const Icon(Icons.fingerprint_rounded),
                title: const Text('Biometric History'),
                subtitle: const Text('View face and biometric entry logs'),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () =>
                    GoRouter.of(context).pushNamed(Paths.biometricHistory),
              ),
            ],
          ),
          FTileGroup(
            label: const Text("VTOP"),
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
                prefix: const Icon(FLucideIcons.anchor),
                title: const Text("General Outing"),
                subtitle: const Text("Open general outing in VTOP"),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => openVtop(_generalOutingUrl),
              ),
              FTile(
                prefix: const Icon(FLucideIcons.amphora),
                title: const Text("Weekend Outing"),
                subtitle: const Text("Open weekend outing in VTOP"),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => openVtop(_weekendOutingUrl),
              ),
            ],
          ),
          if (Platform.isAndroid)
            FTileGroup(
              label: const Text("Browser Extension"),
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.puzzle),
                  title: const Text("Chrome Extension"),
                  subtitle: const Text(
                    "Install VITAP Mate auto login on your computer",
                  ),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () =>
                      GoRouter.of(context).pushNamed(Paths.chromeExtension),
                ),
              ],
            ),
        ],
      ),
    );
  }
}
