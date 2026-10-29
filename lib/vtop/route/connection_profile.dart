import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/vtop/route/vtop_udp.dart';

/// How the app reaches VTOP, as one choice in Settings → Connection. A
/// profile sets the VTOP route and UDP mode, with the default timeouts;
/// [custom] is settings changed by hand under Advanced.
enum ConnectionProfile { phoneOnly, compress, server, custom }

const _presets = <(ConnectionProfile, VtopServerMode)>[
  (.phoneOnly, .off),
  (.compress, .compress),
  (.server, .server),
];

/// The profile the saved settings match, else [ConnectionProfile.custom].
final connectionProfileProvider = Provider<ConnectionProfile>((ref) {
  final mode = ref.watch(vtopModeProvider);
  // UDP matters only when the server fetches; there it is on.
  final udpAsSet = mode != .server || ref.watch(vtopUdpProvider);
  final defaultTimeouts = tunables.every(
    (t) => ref.watch(tunableProvider(t)) == t.fallback,
  );
  if (!udpAsSet || !defaultTimeouts) return .custom;
  return _presets.firstWhere((p) => p.$2 == mode).$1;
});

/// Saves every setting of [profile] (not [ConnectionProfile.custom]).
Future<void> applyConnectionProfile(
  WidgetRef ref,
  ConnectionProfile profile,
) async {
  final (_, mode) = _presets.firstWhere((p) => p.$1 == profile);
  await setVtopMode(ref, mode);
  if (mode == .server) await setVtopUdp(ref, udpModeAvailable);
  for (final tunable in tunables) {
    await setTunable(ref, tunable, tunable.fallback);
  }
}
