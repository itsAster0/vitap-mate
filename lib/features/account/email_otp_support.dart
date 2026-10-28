import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/client_provider.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/vtop/vtop.dart';

/// The registration number Gmail access is checked against, signing in to
/// VTOP first if needed. Toasts and returns null when that fails.
Future<String?> gmailAccountIdentifier(
  BuildContext context,
  WidgetRef ref,
) async {
  var identifier = '';
  try {
    await ref.read(vClientProvider.notifier).ensureLogin();
    identifier = vtopClientRegistrationNumber(
      client: await ref.read(vClientProvider.future),
    ).trim();
  } catch (_) {
    if (context.mounted) {
      dispToast(
        context,
        'VTOP sign-in required',
        'Check your saved VTOP credentials in Settings, sign in once, then retry Gmail setup.',
      );
    }
    return null;
  }
  if (identifier.isEmpty && context.mounted) {
    dispToast(
      context,
      'Setup failed',
      'Open Vitap Mate once with your VTOP account, then retry Gmail setup.',
    );
  }
  return identifier.isEmpty ? null : identifier;
}

/// This phone's VTOP cookies. With [signIn] false, null unless VTOP is
/// already signed in; with it true, signs in (possibly asking for an OTP).
Future<String?> vtopCookies(WidgetRef ref, {required bool signIn}) async {
  try {
    final client = signIn
        ? await ref.read(vClientProvider.notifier).ensureLogin()
        : await ref.read(vClientProvider.future);
    return await signedInCookies(client);
  } catch (_) {
    return null;
  }
}

/// Which steps of a guide the student ticked off, kept in settings under
/// [settingKey] so the guide reopens where they left it.
class GuideSteps {
  const GuideSteps(this.done, this.toggle, this.markDone, this.setAll);

  final Set<int> done;
  final void Function(int step) toggle;

  /// Ticks [step]; safe to call again.
  final void Function(int step) markDone;

  /// Marks the first [count] steps done (0 clears the guide).
  final void Function(int count) setAll;

  /// The first step not done yet, or null when all [total] are.
  int? firstOpen(int total) {
    for (var step = 0; step < total; step++) {
      if (!done.contains(step)) return step;
    }
    return null;
  }
}

GuideSteps useGuideSteps(WidgetRef ref, String settingKey) {
  final prefs = ref.watch(settingsProvider).value;
  final done = useState<Set<int>?>(null);
  useEffect(() {
    if (prefs != null && done.value == null) {
      done.value = {
        for (final raw in prefs.getStringList(settingKey) ?? const <String>[])
          ?int.tryParse(raw),
      };
    }
    return null;
  }, [prefs]);

  void save(Set<int> next) {
    done.value = next;
    prefs?.setStringList(settingKey, [for (final step in next) '$step']);
  }

  final current = done.value ?? const <int>{};
  return GuideSteps(
    current,
    (step) => save(
      current.contains(step)
          ? ({...current}..remove(step))
          : {...current, step},
    ),
    (step) => save({...current, step}),
    (count) => save({for (var step = 0; step < count; step++) step}),
  );
}
