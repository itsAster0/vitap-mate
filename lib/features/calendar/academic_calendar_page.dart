import 'package:vitapmate/observability/log.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/calendar/widgets/calendar_view.dart';

class AcademicCalendarPage extends HookConsumerWidget {
  const AcademicCalendarPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> refresh() async {
      try {
        await ref.read(academicCalendarProvider.notifier).refresh();
      } catch (e) {
        Log.info('features.calendar', '$e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    // Like the other screens, fetch on open when Auto Refresh is on.
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!context.mounted || !await isAutoRefreshEnabled(ref)) return;
        ref
            .read(academicCalendarProvider.notifier)
            .refresh()
            .catchError(
              (e, st) =>
                  Log.info('features.calendar', 'auto refresh failed: $e'),
            );
      });
      return null;
    }, const []);

    final calendar = ref.watch(academicCalendarProvider);
    return ScreenRefresh(
      onRefresh: refresh,
      tasks: const ['vtop_academic_calendar'],
      child: AnimatedSwitcher(
        duration: Motion.medium,
        child: calendar.when(
          skipLoadingOnRefresh: true,
          skipLoadingOnReload: true,
          data: (data) => data.entries.isEmpty
              ? EmptyState(
                  key: const ValueKey('empty'),
                  icon: FLucideIcons.calendarX,
                  title: 'No calendar yet',
                  message: "VTOP hasn't published this semester's calendar.",
                  action: FButton(
                    variant: .outline,
                    mainAxisSize: .min,
                    onPress: refresh,
                    child: const Text('Check again'),
                  ),
                )
              : CalendarView(
                  key: const ValueKey('data'),
                  data: data,
                  onRefresh: refresh,
                ),
          error: (e, _) => EmptyState(
            key: const ValueKey('error'),
            icon: FLucideIcons.cloudAlert,
            title: "Couldn't load the calendar",
            message: commonErrorMessage(e),
            action: FButton(
              variant: .outline,
              mainAxisSize: .min,
              onPress: refresh,
              child: const Text('Try again'),
            ),
          ),
          loading: () => const Padding(
            key: ValueKey('loading'),
            padding: .all(Space.sm),
            child: Column(
              children: [
                Skeleton(height: 20, radius: Radii.sm),
                SizedBox(height: Space.md),
                Skeleton(height: 340, radius: Radii.lg),
                SizedBox(height: Space.lg),
                SkeletonList(count: 4, height: 52),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
