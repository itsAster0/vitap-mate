import 'package:vitapmate/observability/log.dart';
import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/profile/profile.dart';
import 'package:vitapmate/features/profile/widgets/mentor_tab.dart';
import 'package:vitapmate/features/profile/widgets/profile_info_tab.dart';

enum ProfileTab { profile, mentor }

/// The student's VTOP profile, and their mentor with the mentor's messages.
class ProfilePage extends HookConsumerWidget {
  const ProfilePage({super.key, this.initialTab = ProfileTab.profile});

  final ProfileTab initialTab;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tab = useState(initialTab);
    final colors = context.theme.colors;

    Future<void> refresh() async {
      try {
        await switch (tab.value) {
          .profile => ref.read(profileProvider.notifier).refresh(),
          .mentor => ref.read(mentorProvider.notifier).refresh(),
        };
      } catch (e) {
        Log.info('features.profile', '$e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        for (final notifier in [
          ref.read(profileProvider.notifier).refresh,
          ref.read(mentorProvider.notifier).refresh,
        ]) {
          notifier().catchError((e, st) {
            Log.info('features.profile', 'auto refresh failed: $e');
          });
        }
      });
      return null;
    }, const []);

    return ScreenRefresh(
      onRefresh: refresh,
      tasks: const ['vtop_profile', 'vtop_mentor'],
      child: RefreshIndicator(
        onRefresh: refresh,
        backgroundColor: colors.background,
        color: colors.foreground,
        child: ListView(
          padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.lg),
          physics: const AlwaysScrollableScrollPhysics(),
          children: [
            Segmented<ProfileTab>(
              segments: const [
                (ProfileTab.profile, 'Profile'),
                (ProfileTab.mentor, 'Mentor'),
              ],
              value: tab.value,
              onChanged: (value) => tab.value = value,
            ),
            const SizedBox(height: Space.md),
            AnimatedSwitcher(
              duration: Motion.medium,
              child: switch (tab.value) {
                .profile => const ProfileInfoTab(
                  key: ValueKey(ProfileTab.profile),
                ),
                .mentor => const MentorTab(key: ValueKey(ProfileTab.mentor)),
              },
            ),
          ],
        ),
      ),
    );
  }
}
