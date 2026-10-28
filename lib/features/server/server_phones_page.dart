import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/server/server_page.dart';
import 'package:vitapmate/features/server/server_account.dart';
import 'package:vitapmate/features/server/widgets/icon_box.dart';
import 'package:vitapmate/features/server/widgets/key_dialogs.dart';
import 'package:vitapmate/features/server/widgets/server_format.dart';

/// The phones linked to the account.
class ServerPhonesPage extends HookConsumerWidget {
  const ServerPhonesPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final account = ref.watch(serverAccountProvider);
    final (:busy, :run) = useServerAction(context, ref);
    final actions = ref.read(serverAccountProvider.notifier);
    final current = account.value;

    Future<void> remove(BridgePhone phone) async {
      final name = phone.label ?? 'this phone';
      if (!await confirmAction(
        context,
        phone.thisPhone ? 'Unlink this phone?' : 'Unlink $name?',
        phone.thisPhone
            ? 'Connected apps stop asking this phone for sessions and VTOP data comes to it directly. Your other phones keep working.'
            : 'It stops getting session requests. Link it again from the app on that phone.',
        'Unlink',
      )) {
        return;
      }
      await run('Could not unlink', () => actions.removePhone(phone));
    }

    return ServerSubPage(
      title: 'Linked phones',
      children: [
        SectionHeader(
          title: 'Phones',
          trailing: Text('${current?.phones.length ?? 0}/$maxPhones'),
        ),
        if (account.isLoading)
          const Center(child: FCircularProgress.pinwheel())
        else if (account.hasError)
          FAlert(
            variant: .destructive,
            title: const Text('Could not load your phones'),
            subtitle: Text('${account.error}'),
          )
        else
          FTileGroup(
            divider: .indented,
            children: [
              for (final phone in current?.phones ?? const <BridgePhone>[])
                FTile(
                  prefix: ServerIconBox(
                    icon: FLucideIcons.smartphone,
                    tone: phone.thisPhone
                        ? colors.app.success
                        : colors.app.accentTone,
                  ),
                  title: Text(phone.label ?? 'Phone ${phone.id}'),
                  subtitle: Text(
                    phone.thisPhone
                        ? 'This phone'
                        : 'seen ${ago(phone.seenAt)}',
                  ),
                  suffix: FButton.icon(
                    variant: .ghost,
                    onPress: busy.value ? null : () => remove(phone),
                    child: const Icon(FLucideIcons.unlink),
                  ),
                ),
              FTile(
                prefix: ServerIconBox(
                  icon: FLucideIcons.refreshCw,
                  tone: colors.app.warning,
                ),
                title: Text(
                  current?.linkedHere ?? false
                      ? 'Reconnect this phone'
                      : 'Link this phone',
                ),
                subtitle: Text(
                  current?.linkedHere ?? false
                      ? 'Updates this phone\'s notification token'
                      : 'Adds it next to your other linked phones',
                ),
                onPress: busy.value
                    ? null
                    : () => run(
                        'Could not link',
                        () => actions.link(withCredentials: false),
                      ),
              ),
            ],
          ),
        const ServerNote(
          'A key asks only the phone that created it. Unlink a phone and its keys can no longer reach a phone.',
        ),
      ],
    );
  }
}
