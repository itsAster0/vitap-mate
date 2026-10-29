import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/features/server/server_account.dart';
import 'package:vitapmate/features/server/widgets/icon_box.dart';
import 'package:vitapmate/features/server/widgets/key_dialogs.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';

/// Server sign-in on or off, the same choice as Connected apps' "When the
/// phone is offline": on, the server signs in right away with the sealed
/// password, for this phone and its apps.
class ServerSignInTile extends HookConsumerWidget {
  const ServerSignInTile({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final account = ref.watch(serverAccountProvider);
    final (:busy, :run) = useServerAction(context, ref);
    // The phone's copy until the server answers.
    final on = account.hasValue
        ? signInChoice(account.value) != .phone
        : ref
                  .watch(settingsProvider)
                  .value
                  ?.getBool(bridgeServerSignInSettingKey) ??
              false;
    return FTileGroup(
      children: [
        FTile(
          prefix: ServerIconBox(
            icon: FLucideIcons.keyRound,
            tone: context.theme.colors.app.success,
          ),
          title: const Text('Server sign-in'),
          subtitle: Text(
            on
                ? 'Signs in with the saved password'
                : 'Off: the server keeps no password',
          ),
          suffix: busy.value
              ? const FCircularProgress.pinwheel()
              : FSwitch(
                  value: on,
                  onChange: (value) => run(
                    'Could not change server sign-in',
                    () => ref
                        .read(serverAccountProvider.notifier)
                        .setSignIn(value ? .server : .phone),
                  ),
                ),
        ),
      ],
    );
  }
}

/// Offers server sign-in when it is off, as Data saver wants; true when it
/// is on afterwards. A phone-first choice becomes the server's: this phone
/// signs in through the server anyway.
Future<bool> offerServerSignIn(BuildContext context, WidgetRef ref) async {
  final actions = ref.read(serverAccountProvider.notifier);
  final choice = signInChoice(await ref.read(serverAccountProvider.future));
  if (choice == .server) return true;
  if (choice == .phone &&
      (!context.mounted ||
          !await confirm(
            context,
            title: 'Let the server sign in?',
            body:
                'Your password is sealed on the server, which signs in and reads the OTP from forwarded mail or Gmail. On a weak network, nothing to type and no extra round trips.',
            action: 'Save password',
          ))) {
    return false;
  }
  await actions.setSignIn(.server);
  return true;
}
