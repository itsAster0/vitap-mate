import 'dart:async';

import 'package:flutter/material.dart' show showAdaptiveDialog;
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/theme/app_palette.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/features/settings/presentation/email_otp_support.dart';
import 'package:vitapmate/features/settings/presentation/widgets/email_otp_widgets.dart';

const _vtopSender = 'noreply.sdc@vitap.ac.in';
const _gmailForwardingUrl =
    'https://mail.google.com/mail/u/0/#settings/fwdandpop';
const _gmailFiltersUrl = 'https://mail.google.com/mail/u/0/#settings/filters';
const _stepCount = 3;

const _troubleshooting = <(String, String)>[
  (
    'No confirmation mail arrives',
    'Check the address was pasted exactly, then tap Check now. Gmail can take a minute.',
  ),
  (
    'Forwarding is greyed out or “disabled by your administrator”',
    'Your college account blocks forwarding. Use option 1, your own Google key.',
  ),
  (
    'Gmail opens the app instead of settings',
    'Open the link in Chrome, sign in there and turn on ⋮ → Desktop site.',
  ),
  (
    'Still asked to type the OTP',
    'In Gmail, Settings → Filters: the filter’s From must be exactly $_vtopSender and its action must say “Forward to” your address. A filter that only marks mail as read forwards nothing. Mail sent before the filter is not forwarded.',
  ),
  (
    'New phone or reinstalled the app',
    'Sign in to VTOP on the new phone and open this page: your address moves over by itself. If it doesn’t, tap “I already have an address”.',
  ),
  (
    'VTOP didn’t ask for an OTP',
    'VTOP doesn’t ask on every sign-in. Gmail’s confirmation in step 2 only proves the address; also check in Settings → Filters that the filter says “Forward to” your address.',
  ),
];

String _ago(DateTime time) {
  final elapsed = DateTime.now().difference(time);
  if (elapsed.inMinutes < 1) return 'just now';
  if (elapsed.inHours < 1) return '${elapsed.inMinutes} min ago';
  if (elapsed.inDays < 1) return '${elapsed.inHours} h ago';
  return '${elapsed.inDays} d ago';
}

/// Option 1: Gmail forwards VTOP's OTP mail to `vtop-otp-<id>@aster0.dev`.
/// The relay seals each code to this phone's key; only this phone can open
/// it.
class MailForwardingPage extends HookConsumerWidget {
  const MailForwardingPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final app = colors.app;
    final service = ref.read(mailForwardingServiceProvider);
    final forwarding = useState<MailForwarding?>(null);
    final loaded = useState(false);
    final busy = useState(false);
    final checking = useState(false);
    final confirmation = useState<MailOtpPayload?>(null);
    final devices = useState<List<MailForwardingDevice>?>(null);
    final steps = useGuideSteps(ref, forwardingGuideDoneSettingKey);
    final openStep = useState<int?>(null);
    final current = openStep.value ?? steps.firstOpen(_stepCount);
    // The mail listener below lives for the whole page; it reads the
    // latest steps through this.
    final latestSteps = useRef(steps)..value = steps;

    /// Set below; the edge-rejected case joins the address again.
    final rejoin = useRef<Future<void> Function()?>(null);

    Future<void> loadDevices() async {
      if (forwarding.value == null) {
        devices.value = null;
        return;
      }
      try {
        devices.value = await service.devices();
      } on MailForwardingException catch (error) {
        // Another install took the address before addresses could have
        // several devices: this copy's secret is dead. Join it again.
        if (error.mailboxGone) await rejoin.value?.call();
      } catch (_) {
        // The list is extra; the address works without it.
      }
    }

    Future<void> reload() async {
      forwarding.value = await service.load();
      loaded.value = true;
      ref.invalidate(mailForwardingProvider);
      ref.invalidate(anyEmailOtpReadyProvider);
      unawaited(loadDevices());
    }

    Future<void> checkNow({bool quiet = false}) async {
      if (forwarding.value == null || checking.value) return;
      checking.value = true;
      try {
        final payload = await service.pullInbox();
        if (payload == null && !quiet && context.mounted) {
          dispToast(
            context,
            'Nothing yet',
            'No mail has reached your address in the last 5 minutes.',
          );
        }
      } catch (error) {
        if (!quiet && context.mounted) {
          dispToast(context, 'Could not check', '$error');
        }
      } finally {
        if (context.mounted) checking.value = false;
      }
    }

    /// Lets the bridge know whose address this is, once, when VTOP is
    /// already signed in. Best effort: retried on the next visit.
    Future<void> claimOwnerQuietly() async {
      final current = forwarding.value;
      if (current == null || current.ownerClaimed) return;
      final cookies = await vtopCookies(ref, signIn: false);
      if (cookies == null) return;
      try {
        await service.claimOwner(cookies: cookies);
        await reload();
      } catch (_) {}
    }

    /// Moves this student's address to this phone. Null when they have
    /// none, or VTOP isn't signed in ([signIn] false).
    Future<MailForwarding?> reclaimAddress({required bool signIn}) async {
      final cookies = await vtopCookies(ref, signIn: signIn);
      final token = await getFcmTokenForCopy();
      if (cookies == null || token == null || token.isEmpty) return null;
      final found = await service.reclaim(cookies: cookies, fcmToken: token);
      if (found != null) {
        // The address is already set up in Gmail.
        latestSteps.value.setAll(_stepCount);
        await reload();
      }
      return found;
    }

    rejoin.value = () async {
      await service.forgetLocal();
      forwarding.value = null;
      devices.value = null;
      try {
        await reclaimAddress(signIn: false);
      } catch (_) {}
      if (forwarding.value == null) await reload();
    };

    useEffect(() {
      unawaited(() async {
        await reload();
        if (forwarding.value == null) {
          // A new phone: take this student's address back if VTOP is
          // signed in already.
          try {
            final found = await reclaimAddress(signIn: false);
            if (found != null && context.mounted) {
              dispToast(
                context,
                'Your address is back',
                'Forwarded OTPs now come to this device too.',
              );
            }
          } catch (_) {}
        } else {
          await claimOwnerQuietly();
        }
        await checkNow(quiet: true);
      }());
      final subscription = mailOtpEvents.listen((payload) {
        final steps = latestSteps.value;
        if (payload.isGmailConfirmation) {
          confirmation.value = payload;
          // Gmail sent it, so step 1 is done.
          steps.markDone(0);
        } else if (payload.isOtp) {
          unawaited(reload());
          steps.setAll(_stepCount);
        }
      });
      final lifecycle = AppLifecycleListener(
        onResume: () => unawaited(checkNow(quiet: true)),
      );
      return () {
        unawaited(subscription.cancel());
        lifecycle.dispose();
      };
    }, const []);

    Future<String?> pushToken() async {
      final token = await getFcmTokenForCopy();
      if (token != null && token.isNotEmpty) return token;
      if (context.mounted) {
        dispToast(
          context,
          'Notifications needed',
          'Vitap Mate could not get a push token. Allow notifications and try again.',
        );
      }
      return null;
    }

    Future<void> recover() async {
      if (busy.value) return;
      busy.value = true;
      try {
        final found = await reclaimAddress(signIn: true);
        if (!context.mounted) return;
        dispToast(
          context,
          found == null ? 'No address found' : 'Your address is back',
          found == null
              ? 'Your VTOP account has no forwarding address yet. Create one.'
              : 'Forwarded OTPs now come to this device too.',
        );
      } catch (error) {
        if (context.mounted) {
          dispToast(context, 'Could not look it up', '$error');
        }
      } finally {
        if (context.mounted) busy.value = false;
      }
    }

    Future<void> create() async {
      if (busy.value) return;
      busy.value = true;
      try {
        final token = await pushToken();
        if (token == null) return;
        await service.register(fcmToken: token);
        steps.setAll(0);
        await reload();
        await claimOwnerQuietly();
      } catch (error) {
        if (context.mounted) {
          dispToast(context, 'Could not create the address', '$error');
        }
      } finally {
        if (context.mounted) busy.value = false;
      }
    }

    /// Other devices on the address get a choice: leave it to them, or
    /// delete it for everyone.
    Future<void> remove() async {
      final others = (devices.value ?? const <MailForwardingDevice>[])
          .where((d) => !d.thisDevice)
          .length;
      final choice = await showAdaptiveDialog<bool>(
        context: context,
        builder: (dialogContext) => AppDialog(
          title: Text(
            others == 0
                ? 'Remove your forwarding address?'
                : 'Remove the address from this device?',
          ),
          body: Text(
            others == 0
                ? 'Mail forwarded to it is rejected from now on. Also delete the Gmail filter so Gmail stops trying.'
                : 'Your ${others == 1 ? 'other device keeps' : '$others other devices keep'} getting OTPs. '
                      'Delete the address to stop forwarding everywhere; then also delete the Gmail filter.',
          ),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
            if (others > 0)
              FButton(
                variant: FButtonVariant.outline,
                onPress: () => Navigator.of(dialogContext).pop(false),
                child: const Text('Remove from this device'),
              ),
            FButton(
              variant: FButtonVariant.destructive,
              onPress: () => Navigator.of(dialogContext).pop(true),
              child: Text(others == 0 ? 'Remove' : 'Delete everywhere'),
            ),
          ],
        ),
      );
      if (choice == null) return;
      busy.value = true;
      try {
        await service.remove(everywhere: choice);
        steps.setAll(0);
        confirmation.value = null;
        await reload();
      } catch (error) {
        if (context.mounted) dispToast(context, 'Could not remove', '$error');
      } finally {
        if (context.mounted) busy.value = false;
      }
    }

    if (!loaded.value) {
      return const Center(child: FCircularProgress());
    }

    final privacy = GuideCallout(
      icon: FLucideIcons.lock,
      tone: app.success,
      child: const Text(
        'Only VTOP’s OTP mail is forwarded. The relay locks the code to this phone the moment it arrives and keeps nothing readable: our server can’t see your OTP.',
      ),
    );

    final address = forwarding.value;
    if (address == null) {
      return SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(
          Space.page,
          Space.sm,
          Space.page,
          Space.xl,
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Option 1 · about 2 minutes, once. Gmail forwards VTOP’s OTP mail to a private address, and the code reaches this phone in seconds.',
              style: typography.body.sm.copyWith(color: colors.mutedForeground),
            ),
            const SizedBox(height: Space.lg),
            for (final (icon, text) in const [
              (FLucideIcons.atSign, 'You get your own address'),
              (
                FLucideIcons.filter,
                'A Gmail filter forwards only VTOP OTP mail',
              ),
              (
                FLucideIcons.smartphone,
                'The code fills in by itself, even with the app closed',
              ),
            ])
              Padding(
                padding: const EdgeInsets.only(bottom: Space.md),
                child: Row(
                  children: [
                    Container(
                      width: 32,
                      height: 32,
                      decoration: BoxDecoration(
                        color: app.lab.subtle,
                        borderRadius: BorderRadius.circular(Radii.sm),
                      ),
                      child: Icon(icon, size: 16, color: app.lab.onSubtle),
                    ),
                    const SizedBox(width: Space.md),
                    Expanded(child: Text(text, style: typography.body.sm)),
                  ],
                ),
              ),
            const SizedBox(height: Space.xs),
            privacy,
            const SizedBox(height: Space.sm),
            GuideCallout(
              icon: FLucideIcons.circleAlert,
              tone: app.warning,
              child: const Text(
                'Works only in the app, not for server sign-in. Some college accounts don’t allow forwarding; the guide shows how to check.',
              ),
            ),
            const SizedBox(height: Space.xl),
            FButton(
              onPress: busy.value ? null : create,
              prefix: busy.value
                  ? const FCircularProgress(
                      size: FCircularProgressSizeVariant.sm,
                    )
                  : const Icon(FLucideIcons.plus),
              child: const Text('Create my address'),
            ),
            const SizedBox(height: Space.sm),
            FButton(
              variant: FButtonVariant.ghost,
              onPress: busy.value ? null : recover,
              child: const Text('I already have an address'),
            ),
          ],
        ),
      );
    }

    final confirm = confirmation.value;
    final confirmationBox = confirm == null
        ? Row(
            children: [
              const FCircularProgress(size: FCircularProgressSizeVariant.xs),
              const SizedBox(width: Space.sm),
              Expanded(
                child: Text(
                  'Waiting for Gmail’s confirmation mail…',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ),
              FButton(
                variant: FButtonVariant.ghost,
                size: FButtonSizeVariant.sm,
                mainAxisSize: MainAxisSize.min,
                onPress: checking.value ? null : () => checkNow(),
                child: const Text('Check now'),
              ),
            ],
          )
        : Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              GuideCallout(
                icon: FLucideIcons.mailCheck,
                tone: app.success,
                child: Text(
                  confirm.requester == null
                      ? 'Gmail’s confirmation arrived.'
                      : 'Gmail’s confirmation for ${confirm.requester} arrived.',
                ),
              ),
              if (confirm.code case final code?) ...[
                const SizedBox(height: Space.sm),
                CopyValue(label: 'Confirmation code', value: code, large: true),
              ],
              if (confirm.link case final link?) ...[
                const SizedBox(height: Space.sm),
                FButton(
                  onPress: () => openExternalLink(context, link),
                  suffix: const Icon(FLucideIcons.externalLink),
                  child: const Text('Confirm in Gmail'),
                ),
              ],
            ],
          );

    final lastOtp = address.lastOtpAt;
    // Gmail's confirmation (or any OTP) reaching the phone proves the
    // whole path works; VTOP doesn't ask for an OTP every time.
    final ready = lastOtp != null || confirm != null || steps.done.contains(1);
    final (statusOk, statusText) = switch ((lastOtp, ready)) {
      (final at?, _) => (true, 'Working · last OTP ${_ago(at)}'),
      (null, true) => (true, 'Ready · no OTP forwarded yet'),
      _ => (false, 'Not confirmed yet · finish steps 1 and 2'),
    };
    final stepsList = <Widget>[
      GuideStepTile(
        number: 1,
        title: 'Add your address in Gmail',
        done: steps.done.contains(0),
        open: current == 0,
        onTap: () => openStep.value = 0,
        onToggleDone: () => steps.toggle(0),
        actions: const [
          'Open the link in Chrome and turn on ⋮ → Desktop site.',
          'Sign in to the Gmail where VTOP’s OTP mails arrive (usually your @vitapstudent.ac.in).',
          'Tap Add a forwarding address, paste your address, then Next → Proceed → OK.',
        ],
        warning:
            'Leave “Disable forwarding” selected. The filter in step 3 forwards only VTOP mail.',
        doneWhen: 'Gmail says it sent a confirmation code.',
        link: const GuideLink(_gmailForwardingUrl),
      ),
      GuideStepTile(
        number: 2,
        title: 'Confirm the address',
        done: steps.done.contains(1),
        open: current == 1,
        onTap: () => openStep.value = 1,
        onToggleDone: () => steps.toggle(1),
        actions: const [
          'Gmail’s confirmation mail comes here, not to your inbox.',
          'Tap Confirm in Gmail, or paste the code into Gmail’s Verify box.',
        ],
        extra: confirmationBox,
        doneWhen: 'Gmail lists your vtop-otp address as verified.',
      ),
      GuideStepTile(
        number: 3,
        title: 'Create the filter',
        done: steps.done.contains(2),
        open: current == 2,
        onTap: () => openStep.value = 2,
        onToggleDone: () => steps.toggle(2),
        actions: const [
          'Tap Create a new filter.',
          'Paste the sender below into From, then tap Create filter.',
          'Tick Forward it to and pick your vtop-otp address.',
          'Optional: tick Skip the Inbox and Mark as read.',
          'Tap Create filter.',
        ],
        extra: const CopyValue(label: 'From', value: _vtopSender),
        doneWhen: 'the filter shows in Filters and Blocked Addresses.',
        link: const GuideLink(_gmailFiltersUrl),
      ),
    ];

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        Space.page,
        Space.sm,
        Space.page,
        Space.xl,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          CopyValue(label: 'Your address', value: address.address),
          const SizedBox(height: Space.sm),
          Row(
            children: [
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: statusOk ? app.success.base : app.warning.base,
                ),
              ),
              const SizedBox(width: Space.sm),
              Text(
                statusText,
                style: typography.body.xs.copyWith(
                  color: colors.mutedForeground,
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.lg),
          GuideProgress(done: steps.done, total: _stepCount, current: current),
          const SizedBox(height: Space.md),
          for (final step in stepsList) ...[
            step,
            const SizedBox(height: Space.sm),
          ],
          if (devices.value case final list? when list.isNotEmpty) ...[
            const SizedBox(height: Space.md),
            _DeviceList(
              devices: list,
              onRemove: (device) async {
                try {
                  await service.removeDevice(device.deviceId);
                  await loadDevices();
                } catch (error) {
                  if (context.mounted) {
                    dispToast(context, 'Could not remove', '$error');
                  }
                }
              },
            ),
          ],
          const SizedBox(height: Space.md),
          const GuideTroubleshooting(items: _troubleshooting),
          const SizedBox(height: Space.lg),
          privacy,
          const SizedBox(height: Space.lg),
          FButton(
            variant: FButtonVariant.outline,
            onPress: busy.value ? null : remove,
            prefix: Icon(FLucideIcons.trash2, color: app.danger.base),
            child: Text(
              'Remove address',
              style: TextStyle(color: app.danger.base),
            ),
          ),
        ],
      ),
    );
  }
}

/// The devices that get this address's OTPs. Another one is added by
/// opening this page there and choosing "I already have an address".
class _DeviceList extends StatelessWidget {
  const _DeviceList({required this.devices, required this.onRemove});

  final List<MailForwardingDevice> devices;
  final Future<void> Function(MailForwardingDevice device) onRemove;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          devices.length == 1
              ? 'OTPs go to this device'
              : 'OTPs go to ${devices.length} devices',
          style: typography.body.sm.copyWith(fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: Space.xs),
        Text(
          'To add another, open this page on it and tap “I already have an address”.',
          style: typography.body.xs.copyWith(color: colors.mutedForeground),
        ),
        const SizedBox(height: Space.sm),
        FTileGroup(
          children: [
            for (final device in devices)
              FTile(
                prefix: Icon(
                  device.thisDevice
                      ? FLucideIcons.smartphone
                      : FLucideIcons.monitorSmartphone,
                ),
                title: Text(
                  device.thisDevice
                      ? '${device.label ?? 'This device'} (this device)'
                      : device.label ?? 'Another device',
                ),
                subtitle: Text(
                  'Added ${_ago(device.addedAt)} · last used ${_ago(device.seenAt)}',
                ),
                suffix: device.thisDevice
                    ? null
                    : FButton.icon(
                        onPress: () => onRemove(device),
                        child: const Icon(FLucideIcons.x),
                      ),
              ),
          ],
        ),
      ],
    );
  }
}
