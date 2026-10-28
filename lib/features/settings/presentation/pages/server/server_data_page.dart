import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/edge/server_status.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/settings/presentation/providers/server_account.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/icon_box.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/key_dialogs.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/server_format.dart';

const neverKeptText =
    'Never kept: attendance, marks, timetable or any other VTOP data. Your own data stays on this phone. A sealed password is kept only if you let the server sign in for connected apps.';

String _date(DateTime at) {
  final local = at.toLocal();
  return '${local.day}/${local.month}/${local.year}';
}

/// What the server keeps for this account, and deleting it.
class ServerDataPage extends HookConsumerWidget {
  const ServerDataPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final status = ref.watch(serverStatusProvider);
    final (:busy, :run) = useServerAction(context, ref);

    Future<void> clearSession() async {
      if (!await confirmAction(
        context,
        'Clear the server session?',
        'The server forgets your VTOP session. The next refresh signs in again.',
        'Clear',
      )) {
        return;
      }
      await run(
        'Could not clear it',
        () => ref.read(serverStatusServiceProvider).clearSession(),
      );
    }

    Future<void> deleteData() async {
      if (!await confirmAction(
        context,
        'Delete your data?',
        'Removes your keys, linked phones, session and any saved password from the server. Connected apps stop working and VTOP data comes to this phone directly.',
        'Delete',
      )) {
        return;
      }
      await run(
        'Could not delete',
        () => ref.read(serverAccountProvider.notifier).deleteData(),
      );
    }

    final s = status.value;
    FTile item(IconData icon, String title, String value, {Tone? tone}) =>
        FTile(
          prefix: ServerIconBox(
            icon: icon,
            tone: tone ?? colors.app.accentTone,
          ),
          title: Text(title),
          subtitle: Text(value),
        );

    return ServerSubPage(
      title: 'Your data on the server',
      children: [
        const SectionHeader(title: 'What it keeps'),
        if (status.isLoading)
          const Center(child: FCircularProgress.pinwheel())
        else if (status.hasError)
          FAlert(
            variant: FAlertVariant.destructive,
            title: const Text('Could not reach the server'),
            subtitle: Text('${status.error}'),
          )
        else if (s != null)
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              item(
                FLucideIcons.cookie,
                'Sealed VTOP session',
                s.sessionSavedAt == null
                    ? 'None'
                    : '${s.sessionSource == 'server' ? 'made by the server' : 'made by this phone'} · ${ago(s.sessionSavedAt!)}',
              ),
              item(
                FLucideIcons.keyRound,
                'Sealed password',
                s.vaultSavedAt == null
                    ? 'None'
                    : [
                        'saved ${ago(s.vaultSavedAt!)}',
                        s.vaultExpiresAt == null
                            ? 'kept until VTOP rejects it'
                            : 'forgotten on ${_date(s.vaultExpiresAt!)}',
                      ].join(' · '),
              ),
              item(
                FLucideIcons.mail,
                'OTP forwarding address',
                s.forwarding ?? 'Not set up',
              ),
              if (s.vaultSavedAt != null && s.vaultGmail)
                item(
                  FLucideIcons.inbox,
                  'Gmail access',
                  s.forwarding == null
                      ? 'sealed with the password; the server reads the OTP from Gmail'
                      : 'sealed with the password; used only without forwarding',
                ),
              item(FLucideIcons.key, 'Access keys', '${s.keys.length}'),
              item(FLucideIcons.smartphone, 'Linked phones', '${s.phones}'),
            ],
          ),
        const ServerNote(neverKeptText),
        const SectionHeader(title: 'Manage'),
        FTileGroup(
          divider: FItemDivider.indented,
          children: [
            FTile(
              prefix: ServerIconBox(
                icon: FLucideIcons.eraser,
                tone: colors.app.warning,
              ),
              title: const Text('Clear server session'),
              subtitle: const Text('The next refresh signs in again'),
              onPress: busy.value || s?.sessionSavedAt == null
                  ? null
                  : clearSession,
            ),
            FTile(
              prefix: ServerIconBox(
                icon: FLucideIcons.trash2,
                tone: colors.app.danger,
              ),
              title: Text(
                'Delete my data',
                style: TextStyle(color: colors.app.danger.base),
              ),
              subtitle: const Text('Keys, phones, session, password'),
              onPress: busy.value ? null : deleteData,
            ),
          ],
        ),
      ],
    );
  }
}
