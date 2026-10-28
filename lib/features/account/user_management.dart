import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/clinet_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/vtop/vtop_login_with_otp.dart';
import 'package:vitapmate/vtop/users/vtop_users_utils.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/features/exams/exams.dart';
import 'package:vitapmate/vtop/semesters.dart';
import 'package:vitapmate/features/account/widgets/user_card.dart';
import 'package:vitapmate/vtop/http/vtop_platform_http.dart';

class UserBox extends HookConsumerWidget {
  const UserBox({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        try {
          await ref.read(semesterIdProvider.notifier).refresh();
          ref.invalidate(semesterIdProvider);
        } catch (e, st) {
          log('auto refresh failed: $e', stackTrace: st);
        }
      });
      return null;
    }, const []);

    return ref
        .watch(vtopUserProvider)
        .when(
          data: (data) => (data.username ?? '').isEmpty
              ? const SizedBox.shrink()
              : UserCard(user: data),
          error: (e, _) => Center(
            child: Text(
              'Unable to load account details',
              style: TextStyle(color: context.theme.colors.destructive),
            ),
          ),
          loading: () => Center(
            child: SizedBox(
              width: 100,
              height: 100,
              child: CircularProgressIndicator(
                color: context.theme.colors.primary,
              ),
            ),
          ),
        );
  }
}

class SemesterDialog extends HookConsumerWidget {
  final VtopUserEntity user;
  final BuildContext outerContext;

  const SemesterDialog({
    super.key,
    required this.user,
    required this.outerContext,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final busy = useState(false);
    // The semester being saved, for its row's spinner.
    final saving = useState<String?>(null);
    final isRefreshing = useState(false);
    final semester = ref.watch(semesterIdProvider);

    Future<void> handleRefresh() async {
      if (isRefreshing.value) return;
      isRefreshing.value = true;
      try {
        await ref.read(semesterIdProvider.notifier).refresh();
        ref.invalidate(semesterIdProvider);
      } catch (e) {
        if (context.mounted) Navigator.of(context).pop();
        if (outerContext.mounted) disCommonToast(outerContext, e);
      } finally {
        isRefreshing.value = false;
      }
    }

    Future<void> handleSemesterChange(String selected) async {
      if (busy.value) return;
      if (selected == user.semid) return Navigator.of(context).pop();
      busy.value = true;
      saving.value = selected;
      try {
        await ref
            .read(vtopusersutilsProvider.notifier)
            .vtopUserSave(user.withSemester(selected));

        ref
          ..invalidate(vtopUserProvider)
          ..invalidate(vClientProvider);
        if (ref.read(classReminderSettingsProvider).enabled) {
          await ref.read(timetableProvider.notifier).refresh();
        }
        if (ref.read(examReminderSettingsProvider).enabled) {
          await ref.read(examScheduleProvider.notifier).refresh();
        }
        if (context.mounted) Navigator.of(context).pop();
      } finally {
        busy.value = false;
        if (context.mounted) saving.value = null;
      }
    }

    final cancel = FButton(
      variant: .outline,
      onPress: () => Navigator.of(context).pop(),
      child: const Text('Cancel'),
    );
    Widget status(Widget child) => AppDialog(
      title: const Text('Semesters'),
      body: Container(
        decoration: BoxDecoration(color: context.theme.colors.secondary),
        child: child,
      ),
      actions: [cancel],
    );

    return semester.when(
      data: (data) => AppDialog(
        title: Row(
          mainAxisSize: .max,
          children: [
            const Expanded(child: Text('Semester')),
            if (!isRefreshing.value)
              FTappable(
                onPress: handleRefresh,
                child: const Icon(FLucideIcons.rotateCcw),
              )
            else
              const FCircularProgress.pinwheel(),
          ],
        ),
        body: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 400),
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: .stretch,
              children: [
                Text(
                  'Pick which semester the app shows.',
                  style: context.theme.typography.body.sm.copyWith(
                    color: context.theme.colors.mutedForeground,
                  ),
                ),
                for (final group in _groupByYear(data.semesters)) ...[
                  Padding(
                    padding: const .only(top: 14, bottom: 6),
                    child: Text(
                      group.year,
                      style: context.theme.typography.body.xs.copyWith(
                        fontWeight: .w600,
                        letterSpacing: 0.6,
                        color: context.theme.colors.mutedForeground,
                      ),
                    ),
                  ),
                  for (final sem in group.items)
                    Padding(
                      padding: const .only(bottom: 6),
                      child: _SemesterRow(
                        semester: sem,
                        selected: sem.id == user.semid,
                        saving: saving.value == sem.id,
                        enabled: !busy.value,
                        onPress: () => handleSemesterChange(sem.id),
                      ),
                    ),
                ],
              ],
            ),
          ),
        ),
        actions: [if (!busy.value) cancel],
      ),
      error: (e, _) => status(Text('$e')),
      loading: () => status(const FCircularProgress()),
    );
  }
}

/// A semester name split into season, academic year and any extra note:
/// "Winter Semester 2024-25 Freshers" → (Winter, 2024-25, Freshers).
({String season, String year, String note}) _parseSemester(String name) {
  final m = RegExp(
    r'^\s*(\w+)\s+semester\s+(\d{4}-\d{2,4})\s*(.*)$',
    caseSensitive: false,
  ).firstMatch(name);
  if (m == null) return (season: name.trim(), year: '', note: '');
  return (season: m.group(1)!, year: m.group(2)!, note: m.group(3)!.trim());
}

/// Semesters grouped by academic year, keeping VTOP's order (newest first).
List<({String year, List<SemesterInfo> items})> _groupByYear(
  List<SemesterInfo> semesters,
) {
  final groups = <String, List<SemesterInfo>>{};
  for (final s in semesters) {
    final year = _parseSemester(s.name).year;
    groups.putIfAbsent(year.isEmpty ? 'OTHER' : year, () => []).add(s);
  }
  return [for (final e in groups.entries) (year: e.key, items: e.value)];
}

class _SemesterRow extends StatelessWidget {
  const _SemesterRow({
    required this.semester,
    required this.selected,
    required this.saving,
    required this.enabled,
    required this.onPress,
  });

  final SemesterInfo semester;
  final bool selected;
  final bool saving;
  final bool enabled;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final parsed = _parseSemester(semester.name);
    final season = parsed.season.toLowerCase();
    final icon = switch (season) {
      'fall' => FLucideIcons.leaf,
      'winter' => FLucideIcons.snowflake,
      'summer' => FLucideIcons.sun,
      _ => FLucideIcons.calendar,
    };
    final tone = selected ? colors.app.accentTone : null;

    return PressScale(
      scale: 0.98,
      semanticsLabel: '${semester.name}${selected ? ', selected' : ''}',
      onPress: enabled ? onPress : () {},
      child: AnimatedContainer(
        duration: Motion.medium,
        padding: const .symmetric(horizontal: Space.md, vertical: Space.sm + 2),
        decoration: BoxDecoration(
          color: tone?.subtle ?? colors.secondary,
          borderRadius: .circular(Radii.md),
          border: Border.all(
            color: selected ? colors.app.accent : colors.border,
            width: selected ? 1.5 : 1,
          ),
        ),
        child: Row(
          children: [
            Icon(
              icon,
              size: 18,
              color: tone?.onSubtle ?? colors.mutedForeground,
            ),
            const SizedBox(width: Space.md),
            Expanded(
              child: Text(
                parsed.year.isEmpty
                    ? semester.name
                    : '${parsed.season[0].toUpperCase()}${parsed.season.substring(1).toLowerCase()}',
                style: typography.body.md.copyWith(
                  fontWeight: selected ? .w600 : .w500,
                  color: colors.foreground,
                ),
              ),
            ),
            if (parsed.note.isNotEmpty) ...[
              ToneBadge.neutral(context, parsed.note.toUpperCase()),
              const SizedBox(width: Space.sm),
            ],
            SizedBox(
              width: 18,
              child: saving
                  ? const FCircularProgress(size: .sm)
                  : selected
                  ? Icon(
                      FLucideIcons.check,
                      size: 18,
                      color: colors.app.accentTone.onSubtle,
                    )
                  : null,
            ),
          ],
        ),
      ),
    );
  }
}

class UserPassChange extends HookConsumerWidget {
  final VtopUserEntity user;
  const UserPassChange({super.key, required this.user});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final outerContext = context;
    return Column(
      mainAxisAlignment: .center,
      children: [
        FButton(
          variant: .outline,
          size: .sm,
          prefix: const Icon(FLucideIcons.userPen),
          onPress: () => showAdaptiveDialog(
            context: outerContext,
            builder: (dialogContext) => HookBuilder(
              builder: (context) {
                final usernameController = useTextEditingController(
                  text: user.username ?? '',
                );
                final passwordController = useTextEditingController(
                  text: user.password ?? '',
                );
                final isLoading = useState(false);

                return AppDialog(
                  title: const Text('Update Credentials'),
                  body: Column(
                    mainAxisSize: .min,
                    spacing: 12,
                    children: [
                      FTextField(
                        control: FTextFieldControl.managed(
                          controller: usernameController,
                        ),
                        label: const Text("Username"),
                      ),
                      FTextField.password(
                        control: FTextFieldControl.managed(
                          controller: passwordController,
                        ),
                        obscuringCharacter: '*',
                        label: const Text("New Password"),
                      ),
                    ],
                  ),
                  actions: [
                    if (!isLoading.value)
                      FButton(
                        onPress: () async {
                          final newUsername = usernameController.text.trim();
                          final newPassword = passwordController.text.trim();
                          if (newUsername.isEmpty || newPassword.isEmpty) {
                            return;
                          }

                          isLoading.value = true;
                          try {
                            final client = await createVtopClient(
                              username: newUsername,
                              password: newPassword,
                            );
                            if (!context.mounted) return;
                            await loginWithSecurityOtpPrompt(
                              context: context,
                              client: client,
                            );
                            await ref
                                .read(vtopusersutilsProvider.notifier)
                                .vtopUserSave(
                                  user.withCredentials(
                                    newUsername,
                                    newPassword,
                                  ),
                                );
                            if (user.username != null &&
                                user.username != newUsername) {
                              await clearStoredVtopSession(user.username!);
                            }
                            ref.invalidate(vtopUserProvider);
                            ref.invalidate(vClientProvider);
                          } catch (e) {
                            if (outerContext.mounted) {
                              disOnbardingCommonToast(outerContext, e);
                            }
                          } finally {
                            isLoading.value = false;
                          }

                          if (dialogContext.mounted) {
                            Navigator.of(dialogContext).pop();
                          }
                        },
                        child: const Text('Update'),
                      )
                    else
                      SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: context.theme.colors.primary,
                        ),
                      ),
                    FButton(
                      variant: .outline,
                      onPress: () => Navigator.of(dialogContext).pop(),
                      child: const Text('Cancel'),
                    ),
                  ],
                );
              },
            ),
          ),
          child: const Text('Update details'),
        ),
      ],
    );
  }
}
