import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/route/server_url.dart';
import 'package:vitapmate/vtop/edge/server_status.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/app_card.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/server/server_account.dart';

enum CardState { checking, connected, slow, unreachable, notLinked }

/// What the status card shows, from what is known so far.
CardState statusCardState({required bool linked, ServerHealth? health}) =>
    switch (health) {
      _ when !linked => .notLinked,
      null => .checking,
      ServerHealth(ok: false) => .unreachable,
      ServerHealth(:final rttMs) when rttMs > 1000 => .slow,
      _ => .connected,
    };

class ServerStatusCard extends HookConsumerWidget {
  const ServerStatusCard({super.key, required this.linked});

  final bool linked;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final health = ref.watch(serverHealthProvider).value;
    final notLinked =
        linked && ref.watch(serverStatusProvider).error is ServerNotLinked;
    final state = statusCardState(linked: linked && !notLinked, health: health);
    final busy = useState(false);
    final url = ref.watch(serverUrlProvider);

    final (label, tone) = switch (state) {
      .checking => ('Checking…', colors.mutedForeground),
      .connected => ('Connected', colors.app.success.base),
      .slow => ('Slow', colors.app.warning.base),
      .unreachable => ('Unreachable', colors.app.danger.base),
      .notLinked => ('Not linked', colors.app.warning.base),
    };
    final muted = typography.body.sm.copyWith(color: colors.mutedForeground);

    Future<void> run(Future<void> Function() action) async {
      if (busy.value) return;
      busy.value = true;
      try {
        await action();
      } catch (error) {
        if (context.mounted) {
          dispToast(context, 'Something went wrong', '$error');
        }
      } finally {
        busy.value = false;
      }
    }

    final lines = <String>[
      if (state case .connected || .slow)
        [
          '${health!.rttMs} ms',
          ?health.colo,
          if (health.version case final v?) 'v$v',
        ].join(' · '),
      if (state == .unreachable)
        'Connected apps and compressing VTOP wait until it is back.',
      if (state == .notLinked)
        'Link this phone for connected apps and OTP forwarding.',
    ];

    final Widget? action = switch (state) {
      .notLinked => FButton(
        onPress: busy.value
            ? null
            : () => run(
                () => ref
                    .read(serverAccountProvider.notifier)
                    .link(withCredentials: false),
              ),
        child: const Text('Link this phone'),
      ),
      _ => null,
    };

    return AppCard(
      child: Column(
        crossAxisAlignment: .stretch,
        spacing: Space.xs,
        children: [
          Row(
            children: [
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(color: tone, shape: .circle),
              ),
              const SizedBox(width: Space.sm),
              Text(
                label,
                style: typography.body.md.copyWith(fontWeight: .w600),
              ),
              const SizedBox(width: Space.sm),
              Expanded(
                child: Text(
                  Uri.tryParse(url)?.host ?? url,
                  style: muted,
                  overflow: .ellipsis,
                  textAlign: .end,
                ),
              ),
            ],
          ),
          for (final line in lines) Text(line, style: muted),
          if (action != null) ...[const SizedBox(height: Space.xs), action],
        ],
      ),
    );
  }
}
